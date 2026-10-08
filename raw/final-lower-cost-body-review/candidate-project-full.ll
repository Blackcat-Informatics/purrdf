define void @purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %3, i64 noundef range(i64 0, 576460752303423488) %4, ptr noalias nofree noundef align 16 dereferenceable(1248) %5) unnamed_addr #8 personality ptr @rust_eh_personality !guid !36366 {
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
  %17 = alloca [40 x i8], align 8
  %18 = alloca [24 x i8], align 8
  %19 = alloca [8 x i8], align 8
  %20 = alloca [112 x i8], align 16
  %21 = alloca [104 x i8], align 8
  %22 = alloca [32 x i8], align 8
  %23 = alloca [32 x i8], align 8
  %24 = alloca [32 x i8], align 8
  %25 = alloca [104 x i8], align 8
  %26 = alloca [96 x i8], align 8
  %27 = alloca [104 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %27)
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %27, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36367)
  %28 = getelementptr inbounds nuw i8, ptr %5, i64 1120
  %29 = getelementptr inbounds nuw i8, ptr %5, i64 1136
  %30 = load i64, ptr %29, align 16, !alias.scope !36367, !noalias !36370, !noundef !1740
  %31 = icmp ult i64 %30, 192153584101141163
  tail call void @llvm.assume(i1 %31)
  %32 = icmp eq i64 %30, 0
  br i1 %32, label %33, label %34

33:                                               ; preds = %6
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %20, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %272 unwind label %270, !inline_history !36374

34:                                               ; preds = %6
  %35 = getelementptr inbounds nuw i8, ptr %5, i64 1128
  %36 = load ptr, ptr %35, align 8, !alias.scope !36367, !noalias !36370, !nonnull !1740, !noundef !1740
  %37 = mul nuw nsw i64 %30, 48
  %38 = getelementptr inbounds nuw i8, ptr %36, i64 %37
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !36375
  %39 = shl nuw nsw i64 %4, 4
  %40 = getelementptr inbounds nuw i8, ptr %3, i64 %39
  %41 = icmp eq i64 %4, 0
  br i1 %41, label %42, label %.preheader53

42:                                               ; preds = %34
  %43 = getelementptr inbounds nuw i8, ptr %36, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36382)
  %44 = getelementptr i8, ptr %36, i64 24
  %45 = load ptr, ptr %44, align 8, !noalias !36385
  %46 = getelementptr i8, ptr %36, i64 32
  %47 = load i64, ptr %46, align 8, !noalias !36385
  br label %.loopexit52

.preheader53:                                     ; preds = %34, %73
  %48 = phi ptr [ %49, %73 ], [ %36, %34 ]
  %49 = getelementptr inbounds nuw i8, ptr %48, i64 48
  %50 = getelementptr inbounds nuw i8, ptr %48, i64 24
  %51 = load ptr, ptr %50, align 8, !noalias !36388, !nonnull !1740, !noundef !1740
  %52 = getelementptr i8, ptr %48, i64 32
  %53 = load i64, ptr %52, align 8, !noalias !36388
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36382)
  %54 = getelementptr inbounds nuw i8, ptr %51, i64 16
  br label %55

55:                                               ; preds = %71, %.preheader53
  %56 = phi ptr [ %3, %.preheader53 ], [ %57, %71 ]
  %57 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %58 = load ptr, ptr %56, align 8, !alias.scope !36382, !noalias !36394, !nonnull !1740, !noundef !1740
  %59 = getelementptr i8, ptr %56, i64 8
  %60 = load i64, ptr %59, align 8, !alias.scope !36382, !noalias !36394, !noundef !1740
  %61 = icmp eq ptr %58, %51
  %62 = icmp eq i64 %60, %53
  %63 = xor i1 %62, true
  %64 = or i1 %61, %63
  br i1 %64, label %69, label %65

65:                                               ; preds = %55
  %66 = getelementptr inbounds nuw i8, ptr %58, i64 16
  %67 = tail call i32 @bcmp(ptr nonnull readonly %66, ptr nonnull readonly %54, i64 %53), !alias.scope !36397, !noalias !36401
  %68 = icmp eq i32 %67, 0
  br i1 %68, label %73, label %71

69:                                               ; preds = %55
  %70 = and i1 %61, %62
  br i1 %70, label %73, label %71

71:                                               ; preds = %69, %65
  %72 = icmp eq ptr %57, %40
  br i1 %72, label %.loopexit52, label %55

73:                                               ; preds = %69, %65
  %74 = icmp eq ptr %49, %38
  br i1 %74, label %.thread, label %.preheader53

.thread:                                          ; preds = %73
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !36375
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !36402
  store ptr inttoptr (i64 8 to ptr), ptr %12, align 8, !noalias !36402
  %75 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %76 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 0, ptr %76, align 8, !noalias !36402
  %77 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr inttoptr (i64 8 to ptr), ptr %77, align 8, !noalias !36402
  br label %.loopexit45

.loopexit52:                                      ; preds = %71, %42
  %78 = phi i64 [ %47, %42 ], [ %53, %71 ]
  %79 = phi ptr [ %45, %42 ], [ %51, %71 ]
  %80 = phi ptr [ %43, %42 ], [ %49, %71 ]
  %81 = atomicrmw add ptr %79, i64 1 monotonic, align 8, !noalias !36385
  %82 = icmp slt i64 %81, 0
  br i1 %82, label %83, label %89

83:                                               ; preds = %.loopexit52
  tail call void @llvm.trap()
  unreachable

84:                                               ; preds = %__rustc::__rust_alloc (.exit.thread)
  %85 = landingpad { ptr, i32 }
          cleanup
  %86 = atomicrmw sub ptr %79, i64 1 release, align 8, !noalias !36403
  %87 = icmp eq i64 %86, 1
  br i1 %87, label %88, label %767

88:                                               ; preds = %84
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %8) #92
          to label %767 unwind label %219, !noalias !36375

89:                                               ; preds = %.loopexit52
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !36375
  store ptr %79, ptr %8, align 8, !noalias !36375
  %90 = getelementptr inbounds nuw i8, ptr %8, i64 8
  store i64 %78, ptr %90, align 8, !noalias !36375
  %91 = tail call noundef dereferenceable_or_null(64) ptr @malloc(i64 noundef range(i64 1, 0) 64) #93, !noalias !36410
  %92 = icmp eq ptr %91, null
  br i1 %92, label %__rustc::__rust_alloc (.exit.thread), label %93

93:                                               ; preds = %89
  %94 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %95 = load i64, ptr %94, align 8, !noalias !36410, !noundef !1740
  %96 = tail call i64 @llvm.uadd.sat.i64(i64 %95, i64 1)
  store i64 %96, ptr %94, align 8, !noalias !36410
  %97 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %98 = load i64, ptr %97, align 8, !noalias !36410, !noundef !1740
  %99 = tail call i64 @llvm.uadd.sat.i64(i64 %98, i64 64)
  store i64 %99, ptr %97, align 8, !noalias !36410
  %100 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %101 = load i64, ptr %100, align 8, !noalias !36410, !noundef !1740
  %102 = tail call i64 @llvm.sadd.sat.i64(i64 %101, i64 64)
  store i64 %102, ptr %100, align 8, !noalias !36410
  %103 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %104 = load i64, ptr %103, align 8, !noalias !36410, !noundef !1740
  %105 = icmp sgt i64 %102, %104
  br i1 %105, label %106, label %.preheader296

106:                                              ; preds = %93
  store i64 %102, ptr %103, align 8, !noalias !36410
  br label %.preheader296

.preheader296:                                    ; preds = %106, %93
  br label %107

107:                                              ; preds = %.preheader296, %110
  %108 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36410
  %109 = icmp slt i64 %108, 0
  br i1 %109, label %110, label %__rustc::__rust_alloc (.exit)

110:                                              ; preds = %107
  %111 = add nsw i64 %108, 1
  %112 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %108, i64 %111 acq_rel acquire, align 8, !noalias !36410
  %113 = extractvalue { i64, i1 } %112, 1
  br i1 %113, label %114, label %107

114:                                              ; preds = %110
  %115 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36410
  %116 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 64 monotonic, align 8, !noalias !36410
  %117 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 64 monotonic, align 8, !noalias !36410
  %118 = tail call i64 @llvm.sadd.sat.i64(i64 %117, i64 64)
  %119 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36410
  br label %120

120:                                              ; preds = %123, %114
  %121 = phi i64 [ %119, %114 ], [ %126, %123 ]
  %122 = icmp sgt i64 %118, %121
  br i1 %122, label %123, label %127

123:                                              ; preds = %120
  %124 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %121, i64 %118 monotonic monotonic, align 8, !noalias !36410
  %125 = extractvalue { i64, i1 } %124, 1
  %126 = extractvalue { i64, i1 } %124, 0
  br i1 %125, label %127, label %120

127:                                              ; preds = %123, %120
  %128 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36410
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %89
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 64) #94
          to label %129 unwind label %84, !noalias !36375

129:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %107, %127
  store ptr %79, ptr %91, align 8, !noalias !36375
  %130 = getelementptr inbounds nuw i8, ptr %91, i64 8
  store i64 %78, ptr %130, align 8, !noalias !36375
  store i64 4, ptr %9, align 8, !noalias !36375
  %131 = getelementptr inbounds nuw i8, ptr %9, i64 8
  store ptr %91, ptr %131, align 8, !noalias !36375
  %132 = getelementptr inbounds nuw i8, ptr %9, i64 16
  store i64 1, ptr %132, align 8, !noalias !36375
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !36375
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36413)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36416)
  %133 = icmp eq ptr %80, %38
  br i1 %133, label %.loopexit47, label %134

134:                                              ; preds = %__rustc::__rust_alloc (.exit)
  %135 = getelementptr inbounds nuw i8, ptr %7, i64 8
  br i1 %41, label %.preheader, label %.preheader49

.preheader:                                       ; preds = %134, %153
  %136 = phi ptr [ %154, %153 ], [ %91, %134 ]
  %137 = phi i64 [ %157, %153 ], [ 1, %134 ]
  %138 = phi ptr [ %139, %153 ], [ %80, %134 ]
  %139 = getelementptr inbounds nuw i8, ptr %138, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36419)
  %140 = getelementptr i8, ptr %138, i64 24
  %141 = load ptr, ptr %140, align 8, !noalias !36422, !nonnull !1740, !noundef !1740
  %142 = getelementptr i8, ptr %138, i64 32
  %143 = load i64, ptr %142, align 8, !noalias !36422
  %144 = atomicrmw add ptr %141, i64 1 monotonic, align 8, !noalias !36422
  %145 = icmp slt i64 %144, 0
  br i1 %145, label %.loopexit46, label %146

146:                                              ; preds = %.preheader
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !36427
  store ptr %141, ptr %7, align 8, !noalias !36427
  store i64 %143, ptr %135, align 8, !noalias !36427
  %147 = icmp samesign ult i64 %137, 576460752303423488
  tail call void @llvm.assume(i1 %147)
  %148 = load i64, ptr %9, align 8, !range !1835, !alias.scope !36428, !noalias !36429, !noundef !1740
  %149 = icmp eq i64 %137, %148
  br i1 %149, label %150, label %153

150:                                              ; preds = %146
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %137, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %151 unwind label %159, !noalias !36429

151:                                              ; preds = %150
  %152 = load ptr, ptr %131, align 8, !alias.scope !36428, !noalias !36429
  br label %153

153:                                              ; preds = %151, %146
  %154 = phi ptr [ %152, %151 ], [ %136, %146 ]
  %155 = getelementptr inbounds nuw [16 x i8], ptr %154, i64 %137
  store ptr %141, ptr %155, align 8, !noalias !36427
  %156 = getelementptr inbounds nuw i8, ptr %155, i64 8
  store i64 %143, ptr %156, align 8, !noalias !36427
  %157 = add nuw nsw i64 %137, 1
  store i64 %157, ptr %132, align 8, !alias.scope !36428, !noalias !36429
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !36427
  %158 = icmp eq ptr %139, %38
  br i1 %158, label %.loopexit47, label %.preheader

159:                                              ; preds = %150
  %160 = landingpad { ptr, i32 }
          cleanup
  br label %207

.preheader49:                                     ; preds = %134, %199
  %161 = phi ptr [ %200, %199 ], [ %91, %134 ]
  %162 = phi i64 [ %203, %199 ], [ 1, %134 ]
  %163 = phi ptr [ %166, %199 ], [ %80, %134 ]
  br label %164

164:                                              ; preds = %190, %.preheader49
  %165 = phi ptr [ %166, %190 ], [ %163, %.preheader49 ]
  %166 = getelementptr inbounds nuw i8, ptr %165, i64 48
  %167 = getelementptr i8, ptr %165, i64 24
  %168 = load ptr, ptr %167, align 8, !noalias !36430, !nonnull !1740, !noundef !1740
  %169 = getelementptr i8, ptr %165, i64 32
  %170 = load i64, ptr %169, align 8, !noalias !36430
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36419)
  %171 = getelementptr inbounds nuw i8, ptr %168, i64 16
  br label %172

172:                                              ; preds = %188, %164
  %173 = phi ptr [ %3, %164 ], [ %174, %188 ]
  %174 = getelementptr inbounds nuw i8, ptr %173, i64 16
  %175 = load ptr, ptr %173, align 8, !alias.scope !36419, !noalias !36436, !nonnull !1740, !noundef !1740
  %176 = getelementptr i8, ptr %173, i64 8
  %177 = load i64, ptr %176, align 8, !alias.scope !36419, !noalias !36436, !noundef !1740
  %178 = icmp eq ptr %175, %168
  %179 = icmp eq i64 %177, %170
  %180 = xor i1 %179, true
  %181 = or i1 %178, %180
  br i1 %181, label %186, label %182

182:                                              ; preds = %172
  %183 = getelementptr inbounds nuw i8, ptr %175, i64 16
  %184 = tail call i32 @bcmp(ptr nonnull readonly %183, ptr nonnull readonly %171, i64 %170), !alias.scope !36439, !noalias !36443
  %185 = icmp eq i32 %184, 0
  br i1 %185, label %190, label %188

186:                                              ; preds = %172
  %187 = and i1 %178, %179
  br i1 %187, label %190, label %188

188:                                              ; preds = %186, %182
  %189 = icmp eq ptr %174, %40
  br i1 %189, label %192, label %172

190:                                              ; preds = %186, %182
  %191 = icmp eq ptr %166, %38
  br i1 %191, label %.loopexit47, label %164

192:                                              ; preds = %188
  %193 = atomicrmw add ptr %168, i64 1 monotonic, align 8, !noalias !36422
  %194 = icmp slt i64 %193, 0
  br i1 %194, label %.loopexit46, label %195

.loopexit46:                                      ; preds = %192, %.preheader
  tail call void @llvm.trap()
  unreachable

195:                                              ; preds = %192
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !36427
  store ptr %168, ptr %7, align 8, !noalias !36427
  store i64 %170, ptr %135, align 8, !noalias !36427
  %196 = icmp samesign ult i64 %162, 576460752303423488
  tail call void @llvm.assume(i1 %196)
  %197 = load i64, ptr %9, align 8, !range !1835, !alias.scope !36428, !noalias !36429, !noundef !1740
  %198 = icmp eq i64 %162, %197
  br i1 %198, label %213, label %199

199:                                              ; preds = %214, %195
  %200 = phi ptr [ %215, %214 ], [ %161, %195 ]
  %201 = getelementptr inbounds nuw [16 x i8], ptr %200, i64 %162
  store ptr %168, ptr %201, align 8, !noalias !36427
  %202 = getelementptr inbounds nuw i8, ptr %201, i64 8
  store i64 %170, ptr %202, align 8, !noalias !36427
  %203 = add nuw nsw i64 %162, 1
  store i64 %203, ptr %132, align 8, !alias.scope !36428, !noalias !36429
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !36427
  %204 = icmp eq ptr %166, %38
  br i1 %204, label %.loopexit47, label %.preheader49

205:                                              ; preds = %213
  %206 = landingpad { ptr, i32 }
          cleanup
  br label %207

207:                                              ; preds = %205, %159
  %208 = phi ptr [ %168, %205 ], [ %141, %159 ]
  %209 = phi { ptr, i32 } [ %206, %205 ], [ %160, %159 ]
  %210 = atomicrmw sub ptr %208, i64 1 release, align 8, !noalias !36444
  %211 = icmp eq i64 %210, 1
  br i1 %211, label %212, label %218

212:                                              ; preds = %207
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7) #92
          to label %218 unwind label %216, !noalias !36427

213:                                              ; preds = %195
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %162, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %214 unwind label %205, !noalias !36429

214:                                              ; preds = %213
  %215 = load ptr, ptr %131, align 8, !alias.scope !36428, !noalias !36429
  br label %199

216:                                              ; preds = %212
  %217 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !36427
  unreachable

218:                                              ; preds = %212, %207
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
  invoke void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9) #90
          to label %767 unwind label %219, !noalias !36375

219:                                              ; preds = %218, %88
  %220 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !36375
  unreachable

.loopexit47:                                      ; preds = %199, %190, %153, %__rustc::__rust_alloc (.exit)
  %221 = phi i64 [ %162, %190 ], [ %157, %153 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %203, %199 ]
  %222 = load i64, ptr %9, align 8, !noalias !36451
  %223 = load ptr, ptr %131, align 8, !noalias !36451
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !36375
  %224 = icmp ult i64 %221, 576460752303423488
  tail call void @llvm.assume(i1 %224)
  %225 = shl nuw nsw i64 %221, 4
  %226 = getelementptr inbounds nuw i8, ptr %223, i64 %225
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !36402
  store ptr %223, ptr %12, align 8, !noalias !36402
  %227 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %228 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %222, ptr %228, align 8, !noalias !36402
  %229 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr %226, ptr %229, align 8, !noalias !36402
  %230 = getelementptr inbounds nuw i8, ptr %11, i64 24
  %231 = getelementptr inbounds nuw i8, ptr %11, i64 32
  %232 = getelementptr inbounds nuw i8, ptr %11, i64 40
  %233 = load i64, ptr %29, align 16, !alias.scope !36452, !noalias !36455
  br label %235

234:                                              ; preds = %245
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12) #90
          to label %767 unwind label %262, !noalias !36457, !inline_history !36458

235:                                              ; preds = %264, %.loopexit47
  %236 = phi i64 [ %233, %.loopexit47 ], [ %267, %264 ]
  %237 = phi ptr [ %223, %.loopexit47 ], [ %238, %264 ]
  %238 = getelementptr inbounds nuw i8, ptr %237, i64 16
  %239 = load ptr, ptr %237, align 8, !noalias !36459, !nonnull !1740, !noundef !1740
  %240 = getelementptr inbounds nuw i8, ptr %237, i64 8
  %241 = load i64, ptr %240, align 8, !noalias !36459, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !36402
  store ptr %239, ptr %230, align 8, !noalias !36402
  store i64 %241, ptr %231, align 8, !noalias !36402
  store i64 2, ptr %11, align 8, !noalias !36402
  store i8 0, ptr %232, align 8, !noalias !36402
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36452)
  %242 = load i64, ptr %28, align 16, !range !1835, !alias.scope !36452, !noalias !36455, !noundef !1740
  %243 = icmp eq i64 %236, %242
  br i1 %243, label %244, label %264

244:                                              ; preds = %235
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %28)
          to label %264 unwind label %245, !noalias !36455

245:                                              ; preds = %244
  %246 = landingpad { ptr, i32 }
          cleanup
  store ptr %238, ptr %227, align 8
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(48) %11) #90
          to label %234 unwind label %247, !noalias !36462

247:                                              ; preds = %245
  %248 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !36462
  unreachable

.loopexit45:                                      ; preds = %264, %.thread
  %249 = phi ptr [ %75, %.thread ], [ %227, %264 ]
  %250 = phi ptr [ inttoptr (i64 8 to ptr), %.thread ], [ %226, %264 ]
  store ptr %250, ptr %249, align 1
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12)
          to label %251 unwind label %270, !inline_history !36458

251:                                              ; preds = %.loopexit45
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !36402
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !36402
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %10, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %252 unwind label %270, !inline_history !36374

252:                                              ; preds = %251
  %253 = load i64, ptr %29, align 16, !alias.scope !36463, !noalias !36466, !noundef !1740
  %254 = icmp ugt i64 %30, %253
  br i1 %254, label %261, label %255

255:                                              ; preds = %252
  %256 = sub nuw i64 %253, %30
  %257 = load ptr, ptr %35, align 8, !alias.scope !36463, !noalias !36466, !nonnull !1740, !noundef !1740
  %258 = getelementptr inbounds nuw [48 x i8], ptr %257, i64 %30
  store i64 %30, ptr %29, align 16, !alias.scope !36463, !noalias !36466
; invoke core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
  invoke fastcc void @core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>(ptr noalias nofree noundef nonnull align 8 %258, i64 noundef %256)
          to label %261 unwind label %259

259:                                              ; preds = %255
  %260 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef align 16 dereferenceable(112) %10) #90
          to label %767 unwind label %262, !noalias !36466, !inline_history !36458

261:                                              ; preds = %255, %252
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(112) %20, ptr noundef nonnull align 16 dereferenceable(112) %10, i64 112, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !36402
  br label %272

262:                                              ; preds = %259, %234
  %263 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !36466, !inline_history !36458
  unreachable

264:                                              ; preds = %244, %235
  %265 = load ptr, ptr %35, align 8, !alias.scope !36452, !noalias !36455, !nonnull !1740, !noundef !1740
  %266 = getelementptr inbounds nuw [48 x i8], ptr %265, i64 %236
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %266, ptr noundef nonnull align 8 dereferenceable(48) %11, i64 48, i1 false), !noalias !36462
  %267 = add i64 %236, 1
  store i64 %267, ptr %29, align 16, !alias.scope !36452, !noalias !36455
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !36402
  %268 = icmp eq ptr %238, %226
  br i1 %268, label %.loopexit45, label %235

269:                                              ; preds = %369
  br i1 %370, label %767, label %765

270:                                              ; preds = %365, %358, %251, %.loopexit45, %33
  %271 = landingpad { ptr, i32 }
          cleanup
  br label %767

272:                                              ; preds = %261, %33
  %273 = load i64, ptr %20, align 16, !range !1739, !noundef !1740
  %274 = trunc nuw i64 %273 to i1
  br i1 %274, label %275, label %358

275:                                              ; preds = %272
  %276 = getelementptr inbounds nuw i8, ptr %20, i64 16
  %277 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %277, ptr noundef nonnull align 16 dereferenceable(96) %276, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36467)
  %278 = getelementptr inbounds nuw i8, ptr %27, i64 72
  %279 = load i64, ptr %278, align 8, !range !1778, !alias.scope !36470, !noundef !1740
  %280 = icmp ugt i64 %279, 5
  br i1 %280, label %281, label %315

281:                                              ; preds = %275
  %282 = getelementptr inbounds nuw i8, ptr %27, i64 80
  %283 = load ptr, ptr %282, align 8, !alias.scope !36467, !nonnull !1740, !noundef !1740
  %284 = mul i64 %279, 3
  %285 = add i64 %284, -3
  %286 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %287 = load i64, ptr %286, align 8, !noalias !36473, !noundef !1740
  %288 = tail call i64 @llvm.umin.i64(i64 %285, i64 9223372036854775807)
  %289 = tail call i64 @llvm.ssub.sat.i64(i64 %287, i64 %288)
  store i64 %289, ptr %286, align 8, !noalias !36473
  %290 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %291 = load i64, ptr %290, align 8, !noalias !36473, !noundef !1740
  %292 = icmp slt i64 %289, %291
  br i1 %292, label %293, label %.preheader261

293:                                              ; preds = %281
  store i64 %289, ptr %290, align 8, !noalias !36473
  br label %.preheader261

.preheader261:                                    ; preds = %293, %281
  br label %294

294:                                              ; preds = %.preheader261, %297
  %295 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36473
  %296 = icmp slt i64 %295, 0
  br i1 %296, label %297, label %__rustc::__rust_dealloc (.exit)

297:                                              ; preds = %294
  %298 = add nsw i64 %295, 1
  %299 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %295, i64 %298 acq_rel acquire, align 8, !noalias !36473
  %300 = extractvalue { i64, i1 } %299, 1
  br i1 %300, label %301, label %294

301:                                              ; preds = %297
  %302 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %288 monotonic, align 8, !noalias !36473
  %303 = tail call i64 @llvm.ssub.sat.i64(i64 %302, i64 %288)
  %304 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36473
  br label %305

305:                                              ; preds = %308, %301
  %306 = phi i64 [ %304, %301 ], [ %311, %308 ]
  %307 = icmp slt i64 %303, %306
  br i1 %307, label %308, label %312

308:                                              ; preds = %305
  %309 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %306, i64 %303 monotonic monotonic, align 8, !noalias !36473
  %310 = extractvalue { i64, i1 } %309, 1
  %311 = extractvalue { i64, i1 } %309, 0
  br i1 %310, label %312, label %305

312:                                              ; preds = %308, %305
  %313 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36473
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %294, %312
  %314 = icmp ne i64 %285, 0
  tail call void @llvm.assume(i1 %314), !noalias !36473
  tail call void @free(ptr noundef nonnull %283) #93, !noalias !36473
  br label %315

315:                                              ; preds = %__rustc::__rust_dealloc (.exit), %275
  %316 = load i64, ptr %27, align 8, !range !2059, !alias.scope !36467, !noundef !1740
  %317 = icmp sgt i64 %316, 0
  br i1 %317, label %318, label %350

318:                                              ; preds = %315
  %319 = getelementptr inbounds nuw i8, ptr %27, i64 8
  %320 = load ptr, ptr %319, align 8, !alias.scope !36467, !nonnull !1740, !noundef !1740
  %321 = mul nuw i64 %316, 3
  %322 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %323 = load i64, ptr %322, align 8, !noalias !36467, !noundef !1740
  %324 = tail call i64 @llvm.umin.i64(i64 %321, i64 9223372036854775807)
  %325 = tail call i64 @llvm.ssub.sat.i64(i64 %323, i64 %324)
  store i64 %325, ptr %322, align 8, !noalias !36467
  %326 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %327 = load i64, ptr %326, align 8, !noalias !36467, !noundef !1740
  %328 = icmp slt i64 %325, %327
  br i1 %328, label %329, label %.preheader260

329:                                              ; preds = %318
  store i64 %325, ptr %326, align 8, !noalias !36467
  br label %.preheader260

.preheader260:                                    ; preds = %329, %318
  br label %330

330:                                              ; preds = %.preheader260, %333
  %331 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36467
  %332 = icmp slt i64 %331, 0
  br i1 %332, label %333, label %__rustc::__rust_dealloc (.exit38)

333:                                              ; preds = %330
  %334 = add nsw i64 %331, 1
  %335 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %331, i64 %334 acq_rel acquire, align 8, !noalias !36467
  %336 = extractvalue { i64, i1 } %335, 1
  br i1 %336, label %337, label %330

337:                                              ; preds = %333
  %338 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %324 monotonic, align 8, !noalias !36467
  %339 = tail call i64 @llvm.ssub.sat.i64(i64 %338, i64 %324)
  %340 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36467
  br label %341

341:                                              ; preds = %344, %337
  %342 = phi i64 [ %340, %337 ], [ %347, %344 ]
  %343 = icmp slt i64 %339, %342
  br i1 %343, label %344, label %348

344:                                              ; preds = %341
  %345 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %342, i64 %339 monotonic monotonic, align 8, !noalias !36467
  %346 = extractvalue { i64, i1 } %345, 1
  %347 = extractvalue { i64, i1 } %345, 0
  br i1 %346, label %348, label %341

348:                                              ; preds = %344, %341
  %349 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36467
  br label %__rustc::__rust_dealloc (.exit38)

__rustc::__rust_dealloc (.exit38): ; preds = %330, %348
  tail call void @free(ptr noundef nonnull %320) #93, !noalias !36467
  br label %350

350:                                              ; preds = %__rustc::__rust_dealloc (.exit38), %315
  %351 = getelementptr inbounds nuw i8, ptr %27, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36476)
  %352 = load ptr, ptr %351, align 8, !alias.scope !36479, !noundef !1740
  %353 = icmp eq ptr %352, null
  br i1 %353, label %705, label %354

354:                                              ; preds = %350
  %355 = atomicrmw sub ptr %352, i64 1 release, align 8, !noalias !36480
  %356 = icmp eq i64 %355, 1
  br i1 %356, label %357, label %705

357:                                              ; preds = %354
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %351) #92
  br label %705

358:                                              ; preds = %272
  %359 = getelementptr inbounds nuw i8, ptr %20, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %22, ptr noalias nofree noundef align 8 dereferenceable(104) %27, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %359)
          to label %360 unwind label %270

360:                                              ; preds = %358
  %361 = load i64, ptr %22, align 8, !range !2059, !noundef !1740
  %362 = icmp eq i64 %361, -1
  br i1 %362, label %365, label %363

363:                                              ; preds = %360
  call void @llvm.lifetime.start.p0(ptr nonnull %23)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %23, ptr noundef nonnull align 8 dereferenceable(32) %22, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
; invoke <purrdf_sparql_eval::solution::VarSchema>::interned
  %364 = invoke noundef nonnull ptr @<purrdf_sparql_eval::solution::VarSchema>::interned(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3, i64 noundef %4)
          to label %372 unwind label %367

365:                                              ; preds = %360
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
; invoke <purrdf_sparql_eval::solution::VarSchema>::interned
  %366 = invoke noundef nonnull ptr @<purrdf_sparql_eval::solution::VarSchema>::interned(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3, i64 noundef %4)
          to label %708 unwind label %270

367:                                              ; preds = %363
  %368 = landingpad { ptr, i32 }
          cleanup
  br label %369

369:                                              ; preds = %703, %700, %436, %367
  %370 = phi i1 [ true, %367 ], [ false, %436 ], [ false, %703 ], [ false, %700 ]
  %371 = phi { ptr, i32 } [ %368, %367 ], [ %489, %436 ], [ %699, %703 ], [ %699, %700 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23) #90
          to label %269 unwind label %706

372:                                              ; preds = %363
  call void @llvm.lifetime.start.p0(ptr nonnull %21)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %21, ptr noundef nonnull align 8 dereferenceable(104) %27, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36485)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36488)
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  store ptr %364, ptr %19, align 8, !noalias !36490
  %373 = getelementptr inbounds nuw i8, ptr %364, i64 24
  %374 = load ptr, ptr %373, align 8, !noalias !36490, !nonnull !1740, !noundef !1740
  %375 = getelementptr inbounds nuw i8, ptr %364, i64 32
  %376 = load i64, ptr %375, align 8, !noalias !36490, !noundef !1740
  %377 = shl nuw i64 %376, 4
  %378 = icmp eq i64 %376, 0
  br i1 %378, label %.loopexit44, label %379

379:                                              ; preds = %372
  %380 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %377) #93, !noalias !36492
  %381 = icmp eq ptr %380, null
  br i1 %381, label %__rustc::__rust_alloc (.exit39.thread), label %382

382:                                              ; preds = %379
  %383 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %384 = load i64, ptr %383, align 8, !noalias !36492, !noundef !1740
  %385 = tail call i64 @llvm.uadd.sat.i64(i64 %384, i64 1)
  store i64 %385, ptr %383, align 8, !noalias !36492
  %386 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %387 = load i64, ptr %386, align 8, !noalias !36492, !noundef !1740
  %388 = tail call i64 @llvm.uadd.sat.i64(i64 %387, i64 %377)
  store i64 %388, ptr %386, align 8, !noalias !36492
  %389 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %390 = load i64, ptr %389, align 8, !noalias !36492, !noundef !1740
  %391 = tail call i64 @llvm.umin.i64(i64 %377, i64 9223372036854775807)
  %392 = tail call i64 @llvm.sadd.sat.i64(i64 %390, i64 %391)
  store i64 %392, ptr %389, align 8, !noalias !36492
  %393 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %394 = load i64, ptr %393, align 8, !noalias !36492, !noundef !1740
  %395 = icmp sgt i64 %392, %394
  br i1 %395, label %396, label %.preheader273

396:                                              ; preds = %382
  store i64 %392, ptr %393, align 8, !noalias !36492
  br label %.preheader273

.preheader273:                                    ; preds = %396, %382
  br label %397

397:                                              ; preds = %.preheader273, %400
  %398 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36492
  %399 = icmp slt i64 %398, 0
  br i1 %399, label %400, label %__rustc::__rust_alloc (.exit39)

400:                                              ; preds = %397
  %401 = add nsw i64 %398, 1
  %402 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %398, i64 %401 acq_rel acquire, align 8, !noalias !36492
  %403 = extractvalue { i64, i1 } %402, 1
  br i1 %403, label %404, label %397

404:                                              ; preds = %400
  %405 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36492
  %406 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %377 monotonic, align 8, !noalias !36492
  %407 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %391 monotonic, align 8, !noalias !36492
  %408 = tail call i64 @llvm.sadd.sat.i64(i64 %407, i64 %391)
  %409 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36492
  br label %410

410:                                              ; preds = %413, %404
  %411 = phi i64 [ %409, %404 ], [ %416, %413 ]
  %412 = icmp sgt i64 %408, %411
  br i1 %412, label %413, label %417

413:                                              ; preds = %410
  %414 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %411, i64 %408 monotonic monotonic, align 8, !noalias !36492
  %415 = extractvalue { i64, i1 } %414, 1
  %416 = extractvalue { i64, i1 } %414, 0
  br i1 %415, label %417, label %410

417:                                              ; preds = %413, %410
  %418 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36492
  br label %__rustc::__rust_alloc (.exit39)

__rustc::__rust_alloc (.exit39.thread): ; preds = %379
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %377) #94
          to label %419 unwind label %437

419:                                              ; preds = %__rustc::__rust_alloc (.exit39.thread)
  unreachable

__rustc::__rust_alloc (.exit39): ; preds = %397, %417
  %420 = getelementptr inbounds nuw i8, ptr %23, i64 24
  %421 = load ptr, ptr %420, align 8, !noalias !36501, !nonnull !1740, !noundef !1740
  %422 = getelementptr inbounds nuw i8, ptr %421, i64 16
  br label %423

423:                                              ; preds = %427, %__rustc::__rust_alloc (.exit39)
  %424 = phi i64 [ 0, %__rustc::__rust_alloc (.exit39) ], [ %432, %427 ]
  %425 = getelementptr inbounds nuw [16 x i8], ptr %374, i64 %424
; invoke <purrdf_sparql_eval::solution::VarSchema>::index_of
  %426 = invoke fastcc { i64, i64 } @<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.11631829254914579133)(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %422, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %425) #88
          to label %427 unwind label %434, !noalias !36521

427:                                              ; preds = %423
  %428 = extractvalue { i64, i64 } %426, 0
  %429 = extractvalue { i64, i64 } %426, 1
  %430 = getelementptr inbounds nuw [16 x i8], ptr %380, i64 %424
  store i64 %428, ptr %430, align 8, !noalias !36522
  %431 = getelementptr inbounds nuw i8, ptr %430, i64 8
  store i64 %429, ptr %431, align 8, !noalias !36522
  %432 = add nuw i64 %424, 1
  %433 = icmp eq i64 %432, %376
  br i1 %433, label %.loopexit44, label %423

434:                                              ; preds = %423
  %435 = landingpad { ptr, i32 }
          cleanup
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %380, i64 noundef %377, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !36527
  br label %698

436:                                              ; preds = %490, %487
  br i1 %488, label %369, label %698

437:                                              ; preds = %__rustc::__rust_alloc (.exit39.thread)
  %438 = landingpad { ptr, i32 }
          cleanup
  br label %698

.loopexit44:                                      ; preds = %427, %372
  %439 = phi ptr [ inttoptr (i64 8 to ptr), %372 ], [ %380, %427 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !36490
  %440 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %441 = load i64, ptr %440, align 8, !alias.scope !36488, !noalias !36528, !noundef !1740
  %442 = icmp ult i64 %441, 230584300921369396
  tail call void @llvm.assume(i1 %442)
  %443 = mul nuw nsw i64 %441, 40
  %444 = icmp eq i64 %441, 0
  br i1 %444, label %445, label %448

445:                                              ; preds = %.loopexit44
  store i64 0, ptr %18, align 8, !noalias !36490
  %446 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %446, align 8, !noalias !36490
  %447 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 0, ptr %447, align 8, !noalias !36490
  br label %.loopexit

448:                                              ; preds = %.loopexit44
  %449 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %443) #93, !noalias !36529
  %450 = icmp eq ptr %449, null
  br i1 %450, label %__rustc::__rust_alloc (.exit40.thread), label %451

451:                                              ; preds = %448
  %452 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %453 = load i64, ptr %452, align 8, !noalias !36529, !noundef !1740
  %454 = tail call i64 @llvm.uadd.sat.i64(i64 %453, i64 1)
  store i64 %454, ptr %452, align 8, !noalias !36529
  %455 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %456 = load i64, ptr %455, align 8, !noalias !36529, !noundef !1740
  %457 = tail call i64 @llvm.uadd.sat.i64(i64 %456, i64 %443)
  store i64 %457, ptr %455, align 8, !noalias !36529
  %458 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %459 = load i64, ptr %458, align 8, !noalias !36529, !noundef !1740
  %460 = tail call i64 @llvm.sadd.sat.i64(i64 %459, i64 %443)
  store i64 %460, ptr %458, align 8, !noalias !36529
  %461 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %462 = load i64, ptr %461, align 8, !noalias !36529, !noundef !1740
  %463 = icmp sgt i64 %460, %462
  br i1 %463, label %464, label %.preheader272

464:                                              ; preds = %451
  store i64 %460, ptr %461, align 8, !noalias !36529
  br label %.preheader272

.preheader272:                                    ; preds = %464, %451
  br label %465

465:                                              ; preds = %.preheader272, %468
  %466 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36529
  %467 = icmp slt i64 %466, 0
  br i1 %467, label %468, label %__rustc::__rust_alloc (.exit40)

468:                                              ; preds = %465
  %469 = add nsw i64 %466, 1
  %470 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %466, i64 %469 acq_rel acquire, align 8, !noalias !36529
  %471 = extractvalue { i64, i1 } %470, 1
  br i1 %471, label %472, label %465

472:                                              ; preds = %468
  %473 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36529
  %474 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %443 monotonic, align 8, !noalias !36529
  %475 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %443 monotonic, align 8, !noalias !36529
  %476 = tail call i64 @llvm.sadd.sat.i64(i64 %475, i64 %443)
  %477 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36529
  br label %478

478:                                              ; preds = %481, %472
  %479 = phi i64 [ %477, %472 ], [ %484, %481 ]
  %480 = icmp sgt i64 %476, %479
  br i1 %480, label %481, label %485

481:                                              ; preds = %478
  %482 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %479, i64 %476 monotonic monotonic, align 8, !noalias !36529
  %483 = extractvalue { i64, i1 } %482, 1
  %484 = extractvalue { i64, i1 } %482, 0
  br i1 %483, label %485, label %478

485:                                              ; preds = %481, %478
  %486 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36529
  br label %__rustc::__rust_alloc (.exit40)

487:                                              ; preds = %695, %590, %491
  %488 = phi i1 [ false, %491 ], [ false, %695 ], [ true, %590 ]
  %489 = phi { ptr, i32 } [ %492, %491 ], [ %696, %695 ], [ %591, %590 ]
  br i1 %378, label %436, label %490

490:                                              ; preds = %487
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %439, i64 noundef %377, i64 noundef range(i64 1, -9223372036854775807) 8) #93
  br label %436

491:                                              ; preds = %__rustc::__rust_alloc (.exit40.thread)
  %492 = landingpad { ptr, i32 }
          cleanup
  br label %487

__rustc::__rust_alloc (.exit40.thread): ; preds = %448
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %443) #94
          to label %697 unwind label %491, !noalias !36528

__rustc::__rust_alloc (.exit40): ; preds = %465, %485
  store i64 %441, ptr %18, align 8, !noalias !36490
  %493 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr %449, ptr %493, align 8, !noalias !36490
  %494 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 0, ptr %494, align 8, !noalias !36490
  %495 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %496 = load ptr, ptr %495, align 8, !alias.scope !36488, !noalias !36528, !nonnull !1740, !noundef !1740
  %497 = getelementptr inbounds nuw i8, ptr %496, i64 %443
  %498 = icmp ult i64 %376, 576460752303423488
  tail call void @llvm.assume(i1 %498)
  %499 = getelementptr inbounds nuw i8, ptr %17, i64 16
  %500 = icmp samesign ugt i64 %376, 4
  %501 = getelementptr inbounds nuw i8, ptr %439, i64 %377
  %502 = getelementptr inbounds nuw i8, ptr %17, i64 8
  br label %503

503:                                              ; preds = %583, %__rustc::__rust_alloc (.exit40)
  %504 = phi ptr [ %449, %__rustc::__rust_alloc (.exit40) ], [ %584, %583 ]
  %505 = phi i64 [ 0, %__rustc::__rust_alloc (.exit40) ], [ %588, %583 ]
  %506 = phi ptr [ %496, %__rustc::__rust_alloc (.exit40) ], [ %508, %583 ]
  %507 = phi i32 [ undef, %__rustc::__rust_alloc (.exit40) ], [ %524, %583 ]
  %508 = getelementptr inbounds nuw i8, ptr %506, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !36490
  store i64 1, ptr %17, align 8, !noalias !36490
  br i1 %500, label %509, label %510, !prof !1742

509:                                              ; preds = %503
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %17, i64 noundef 0, i64 noundef %376, i1 noundef zeroext true) #92
          to label %511 unwind label %594

510:                                              ; preds = %503
  br i1 %378, label %522, label %511

511:                                              ; preds = %510, %509
  %512 = getelementptr inbounds nuw i8, ptr %506, i64 8
  %513 = getelementptr inbounds nuw i8, ptr %506, i64 16
  br label %514

514:                                              ; preds = %571, %511
  %515 = phi ptr [ %439, %511 ], [ %517, %571 ]
  %516 = phi i32 [ %507, %511 ], [ %552, %571 ]
  %517 = getelementptr inbounds nuw i8, ptr %515, i64 16
  %518 = load i64, ptr %515, align 8, !range !1739, !noalias !36528, !noundef !1740
  %519 = trunc nuw i64 %518 to i1
  br i1 %519, label %535, label %551

520:                                              ; preds = %571
  %521 = load i64, ptr %17, align 8, !noalias !36490
  br label %522

522:                                              ; preds = %520, %510
  %523 = phi i64 [ %521, %520 ], [ 1, %510 ]
  %524 = phi i32 [ %552, %520 ], [ %507, %510 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %16)
  %525 = load ptr, ptr %502, align 8, !noalias !36490
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %16, ptr noundef nonnull align 8 dereferenceable(24) %499, i64 24, i1 false), !noalias !36490
  call void @llvm.experimental.noalias.scope.decl(metadata !36532)
  %526 = load i64, ptr %18, align 8, !range !1835, !alias.scope !36532, !noalias !36535, !noundef !1740
  %527 = icmp eq i64 %505, %526
  br i1 %527, label %528, label %583

528:                                              ; preds = %522
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %18)
          to label %529 unwind label %531, !noalias !36535

529:                                              ; preds = %528
  %530 = load ptr, ptr %493, align 8, !alias.scope !36532, !noalias !36535
  br label %583

531:                                              ; preds = %528
  %532 = landingpad { ptr, i32 }
          cleanup
  %533 = icmp ugt i64 %523, 5
  br i1 %533, label %534, label %695

534:                                              ; preds = %531
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %525) ]
  br label %689

535:                                              ; preds = %514
  %536 = getelementptr inbounds nuw i8, ptr %515, i64 8
  %537 = load i64, ptr %536, align 8, !noalias !36528
  %538 = load i64, ptr %506, align 8, !range !1778, !noalias !36528, !noundef !1740
  %539 = add i64 %538, -1
  %540 = icmp ugt i64 %539, 4
  br i1 %540, label %541, label %545

541:                                              ; preds = %535
  %542 = load ptr, ptr %512, align 8, !noalias !36528, !nonnull !1740, !noundef !1740
  %543 = load i64, ptr %513, align 8, !noalias !36528, !noundef !1740
  %544 = add i64 %543, -1
  br label %545

545:                                              ; preds = %541, %535
  %546 = phi i64 [ %544, %541 ], [ %539, %535 ]
  %547 = phi ptr [ %542, %541 ], [ %512, %535 ]
  %548 = icmp ult i64 %537, %546
  br i1 %548, label %578, label %549

549:                                              ; preds = %545
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %537, i64 noundef %546, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.757) #89
          to label %550 unwind label %596

550:                                              ; preds = %549
  unreachable

551:                                              ; preds = %578, %514
  %552 = phi i32 [ %582, %578 ], [ %516, %514 ]
  %553 = phi i32 [ %580, %578 ], [ 2, %514 ]
  %554 = load i64, ptr %17, align 8, !range !1778, !alias.scope !36537, !noalias !36528, !noundef !1740
  %555 = add i64 %554, -1
  %556 = icmp ugt i64 %555, 4
  %557 = load ptr, ptr %502, align 8, !alias.scope !36537, !noalias !36528, !nonnull !1740
  %558 = select i1 %556, ptr %557, ptr %502
  %559 = select i1 %556, ptr %499, ptr %17
  %560 = call i64 @llvm.umax.i64(i64 %555, i64 4)
  %561 = load i64, ptr %559, align 8, !alias.scope !36537, !noalias !36528, !noundef !1740
  %562 = add i64 %561, -1
  %563 = icmp eq i64 %562, %560
  br i1 %563, label %564, label %571, !prof !1742

564:                                              ; preds = %551
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %17, i64 noundef %560, i64 noundef 1, i1 noundef zeroext true) #92
          to label %565 unwind label %592

565:                                              ; preds = %564
  %566 = load i64, ptr %17, align 8, !range !1778, !alias.scope !36537, !noalias !36528, !noundef !1740
  %567 = icmp ugt i64 %566, 5
  %568 = load ptr, ptr %502, align 8, !alias.scope !36537, !noalias !36528, !nonnull !1740
  %569 = select i1 %567, ptr %568, ptr %502
  %570 = select i1 %567, ptr %499, ptr %17
  br label %571

571:                                              ; preds = %565, %551
  %572 = phi ptr [ %569, %565 ], [ %558, %551 ]
  %573 = phi ptr [ %570, %565 ], [ %559, %551 ]
  %574 = getelementptr inbounds nuw [8 x i8], ptr %572, i64 %562
  store i32 %553, ptr %574, align 4, !noalias !36528
  %575 = getelementptr inbounds nuw i8, ptr %574, i64 4
  store i32 %552, ptr %575, align 4, !noalias !36528
  %576 = add i64 %561, 1
  store i64 %576, ptr %573, align 8, !alias.scope !36537, !noalias !36528
  %577 = icmp eq ptr %517, %501
  br i1 %577, label %520, label %514

578:                                              ; preds = %545
  %579 = getelementptr inbounds nuw [8 x i8], ptr %547, i64 %537
  %580 = load i32, ptr %579, align 4, !range !1785, !noalias !36528, !noundef !1740
  %581 = getelementptr inbounds nuw i8, ptr %579, i64 4
  %582 = load i32, ptr %581, align 4, !noalias !36528
  br label %551

583:                                              ; preds = %529, %522
  %584 = phi ptr [ %530, %529 ], [ %504, %522 ]
  %585 = getelementptr inbounds nuw [40 x i8], ptr %584, i64 %505
  store i64 %523, ptr %585, align 8, !noalias !36540
  %586 = getelementptr inbounds nuw i8, ptr %585, i64 8
  store ptr %525, ptr %586, align 8, !noalias !36540
  %587 = getelementptr inbounds nuw i8, ptr %585, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %587, ptr noundef nonnull align 8 dereferenceable(24) %16, i64 24, i1 false), !noalias !36540
  %588 = add nuw nsw i64 %505, 1
  store i64 %588, ptr %494, align 8, !alias.scope !36532, !noalias !36535
  call void @llvm.lifetime.end.p0(ptr nonnull %16)
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !36490
  %589 = icmp eq ptr %508, %497
  br i1 %589, label %.loopexit, label %503

590:                                              ; preds = %657
  %591 = landingpad { ptr, i32 }
          cleanup
  br label %487

592:                                              ; preds = %564
  %593 = landingpad { ptr, i32 }
          cleanup
  br label %598

594:                                              ; preds = %509
  %595 = landingpad { ptr, i32 }
          cleanup
  br label %598

596:                                              ; preds = %549
  %597 = landingpad { ptr, i32 }
          cleanup
  br label %598

598:                                              ; preds = %596, %594, %592
  %599 = phi { ptr, i32 } [ %593, %592 ], [ %595, %594 ], [ %597, %596 ]
  %600 = load i64, ptr %17, align 8, !range !1778, !alias.scope !19070, !noundef !1740
  %601 = icmp ugt i64 %600, 5
  br i1 %601, label %602, label %695

602:                                              ; preds = %598
  %603 = load ptr, ptr %502, align 8, !nonnull !1740, !noundef !1740
  br label %689

604:                                              ; preds = %703, %698
  %605 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !36485
  unreachable

.loopexit:                                        ; preds = %583, %445
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !36490
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !36490
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %14, ptr noundef nonnull align 8 dereferenceable(104) %21, i64 104, i1 false), !noalias !36541
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !36490
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %13, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false), !noalias !36490
  %606 = getelementptr inbounds nuw i8, ptr %13, i64 24
  store ptr %364, ptr %606, align 8, !noalias !36490
  call void @llvm.experimental.noalias.scope.decl(metadata !36542)
  call void @llvm.experimental.noalias.scope.decl(metadata !36545)
  call void @llvm.experimental.noalias.scope.decl(metadata !36547)
  %607 = load i64, ptr %14, align 8, !range !2059, !alias.scope !36545, !noalias !36549, !noundef !1740
  %608 = icmp eq i64 %607, -1
  br i1 %608, label %610, label %609

609:                                              ; preds = %.loopexit
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %15, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %13, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %21), !noalias !36485
  br label %612

610:                                              ; preds = %.loopexit
  %611 = getelementptr inbounds nuw i8, ptr %15, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %611, ptr noundef nonnull readonly align 8 dereferenceable(32) %13, i64 32, i1 false), !alias.scope !36550, !noalias !36551
  store i64 -1, ptr %15, align 8, !alias.scope !36542, !noalias !36552
  br label %612

612:                                              ; preds = %610, %609
  %613 = getelementptr inbounds nuw i8, ptr %14, i64 72
  %614 = load i64, ptr %613, align 8, !range !1778, !alias.scope !36553, !noalias !36549, !noundef !1740
  %615 = icmp ugt i64 %614, 5
  br i1 %615, label %616, label %650

616:                                              ; preds = %612
  %617 = getelementptr inbounds nuw i8, ptr %14, i64 80
  %618 = load ptr, ptr %617, align 8, !alias.scope !36545, !noalias !36549, !nonnull !1740, !noundef !1740
  %619 = mul i64 %614, 3
  %620 = add i64 %619, -3
  %621 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %622 = load i64, ptr %621, align 8, !noalias !36556, !noundef !1740
  %623 = call i64 @llvm.umin.i64(i64 %620, i64 9223372036854775807)
  %624 = call i64 @llvm.ssub.sat.i64(i64 %622, i64 %623)
  store i64 %624, ptr %621, align 8, !noalias !36556
  %625 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %626 = load i64, ptr %625, align 8, !noalias !36556, !noundef !1740
  %627 = icmp slt i64 %624, %626
  br i1 %627, label %628, label %.preheader264

628:                                              ; preds = %616
  store i64 %624, ptr %625, align 8, !noalias !36556
  br label %.preheader264

.preheader264:                                    ; preds = %628, %616
  br label %629

629:                                              ; preds = %.preheader264, %632
  %630 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36556
  %631 = icmp slt i64 %630, 0
  br i1 %631, label %632, label %__rustc::__rust_dealloc (.exit41)

632:                                              ; preds = %629
  %633 = add nsw i64 %630, 1
  %634 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %630, i64 %633 acq_rel acquire, align 8, !noalias !36556
  %635 = extractvalue { i64, i1 } %634, 1
  br i1 %635, label %636, label %629

636:                                              ; preds = %632
  %637 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %623 monotonic, align 8, !noalias !36556
  %638 = call i64 @llvm.ssub.sat.i64(i64 %637, i64 %623)
  %639 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36556
  br label %640

640:                                              ; preds = %643, %636
  %641 = phi i64 [ %639, %636 ], [ %646, %643 ]
  %642 = icmp slt i64 %638, %641
  br i1 %642, label %643, label %647

643:                                              ; preds = %640
  %644 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %641, i64 %638 monotonic monotonic, align 8, !noalias !36556
  %645 = extractvalue { i64, i1 } %644, 1
  %646 = extractvalue { i64, i1 } %644, 0
  br i1 %645, label %647, label %640

647:                                              ; preds = %643, %640
  %648 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36556
  br label %__rustc::__rust_dealloc (.exit41)

__rustc::__rust_dealloc (.exit41): ; preds = %629, %647
  %649 = icmp ne i64 %620, 0
  call void @llvm.assume(i1 %649), !noalias !36556
  call void @free(ptr noundef nonnull %618) #93, !noalias !36556
  br label %650

650:                                              ; preds = %__rustc::__rust_dealloc (.exit41), %612
  %651 = getelementptr inbounds nuw i8, ptr %14, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !36559), !noalias !36528
  %652 = load ptr, ptr %651, align 8, !alias.scope !36562, !noalias !36549, !noundef !1740
  %653 = icmp eq ptr %652, null
  br i1 %653, label %658, label %654

654:                                              ; preds = %650
  %655 = atomicrmw sub ptr %652, i64 1 release, align 8, !noalias !36563
  %656 = icmp eq i64 %655, 1
  br i1 %656, label %657, label %658

657:                                              ; preds = %654
  fence acquire, !noalias !36528
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %651) #92
          to label %658 unwind label %590

658:                                              ; preds = %657, %654, %650
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !36490
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !36490
  %659 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %659, ptr noundef nonnull align 8 dereferenceable(96) %15, i64 96, i1 false), !noalias !36568
  store i64 0, ptr %0, align 16, !alias.scope !36485, !noalias !36568
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !36490
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !36490
  br i1 %378, label %704, label %660

660:                                              ; preds = %658
  %661 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %662 = load i64, ptr %661, align 8, !noundef !1740
  %663 = call i64 @llvm.umin.i64(i64 %377, i64 9223372036854775807)
  %664 = call i64 @llvm.ssub.sat.i64(i64 %662, i64 %663)
  store i64 %664, ptr %661, align 8
  %665 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %666 = load i64, ptr %665, align 8, !noundef !1740
  %667 = icmp slt i64 %664, %666
  br i1 %667, label %668, label %.preheader263

668:                                              ; preds = %660
  store i64 %664, ptr %665, align 8
  br label %.preheader263

.preheader263:                                    ; preds = %668, %660
  br label %669

669:                                              ; preds = %.preheader263, %672
  %670 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %671 = icmp slt i64 %670, 0
  br i1 %671, label %672, label %__rustc::__rust_dealloc (.exit42)

672:                                              ; preds = %669
  %673 = add nsw i64 %670, 1
  %674 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %670, i64 %673 acq_rel acquire, align 8
  %675 = extractvalue { i64, i1 } %674, 1
  br i1 %675, label %676, label %669

676:                                              ; preds = %672
  %677 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %663 monotonic, align 8
  %678 = call i64 @llvm.ssub.sat.i64(i64 %677, i64 %663)
  %679 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %680

680:                                              ; preds = %683, %676
  %681 = phi i64 [ %679, %676 ], [ %686, %683 ]
  %682 = icmp slt i64 %678, %681
  br i1 %682, label %683, label %687

683:                                              ; preds = %680
  %684 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %681, i64 %678 monotonic monotonic, align 8
  %685 = extractvalue { i64, i1 } %684, 1
  %686 = extractvalue { i64, i1 } %684, 0
  br i1 %685, label %687, label %680

687:                                              ; preds = %683, %680
  %688 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit42)

__rustc::__rust_dealloc (.exit42): ; preds = %669, %687
  call void @free(ptr noundef nonnull %439) #93
  br label %704

689:                                              ; preds = %602, %534
  %690 = phi i64 [ %523, %534 ], [ %600, %602 ]
  %691 = phi ptr [ %525, %534 ], [ %603, %602 ]
  %692 = phi { ptr, i32 } [ %532, %534 ], [ %599, %602 ]
  %693 = shl i64 %690, 3
  %694 = add i64 %693, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %691, i64 noundef %694, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !1740
  br label %695

695:                                              ; preds = %689, %598, %531
  %696 = phi { ptr, i32 } [ %599, %598 ], [ %532, %531 ], [ %692, %689 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %18) #90, !noalias !36528
  br label %487

697:                                              ; preds = %__rustc::__rust_alloc (.exit40.thread)
  unreachable

698:                                              ; preds = %437, %436, %434
  %699 = phi { ptr, i32 } [ %489, %436 ], [ %438, %437 ], [ %435, %434 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %21) #90
          to label %700 unwind label %604, !noalias !36485

700:                                              ; preds = %698
  %701 = atomicrmw sub ptr %364, i64 1 release, align 8, !noalias !36569
  %702 = icmp eq i64 %701, 1
  br i1 %702, label %703, label %369

703:                                              ; preds = %700
  fence acquire, !noalias !36485
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %19) #92
          to label %369 unwind label %604

704:                                              ; preds = %__rustc::__rust_dealloc (.exit42), %658
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  call void @llvm.lifetime.end.p0(ptr nonnull %21)
; call core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  br label %705

705:                                              ; preds = %763, %704, %357, %354, %350
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  ret void

706:                                              ; preds = %767, %369
  %707 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91
  unreachable

708:                                              ; preds = %365
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %25, ptr noundef nonnull align 8 dereferenceable(104) %27, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  %709 = getelementptr inbounds nuw i8, ptr %24, i64 24
  store ptr %366, ptr %709, align 8, !alias.scope !36574
  store i64 0, ptr %24, align 8, !alias.scope !36574
  %710 = getelementptr inbounds nuw i8, ptr %24, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %710, align 8, !alias.scope !36574
  %711 = getelementptr inbounds nuw i8, ptr %24, i64 16
  store i64 0, ptr %711, align 8, !alias.scope !36574
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36577)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36580)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36582)
  %712 = load i64, ptr %25, align 8, !range !2059, !alias.scope !36580, !noalias !36584, !noundef !1740
  %713 = icmp eq i64 %712, -1
  br i1 %713, label %715, label %714

714:                                              ; preds = %708
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %26, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %27)
  br label %717

715:                                              ; preds = %708
  %716 = getelementptr inbounds nuw i8, ptr %26, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %716, ptr noundef nonnull readonly align 8 dereferenceable(32) %24, i64 32, i1 false), !alias.scope !36584, !noalias !36580
  store i64 -1, ptr %26, align 8, !alias.scope !36577, !noalias !36585
  br label %717

717:                                              ; preds = %715, %714
  %718 = getelementptr inbounds nuw i8, ptr %25, i64 72
  %719 = load i64, ptr %718, align 8, !range !1778, !alias.scope !36586, !noalias !36584, !noundef !1740
  %720 = icmp ugt i64 %719, 5
  br i1 %720, label %721, label %755

721:                                              ; preds = %717
  %722 = getelementptr inbounds nuw i8, ptr %25, i64 80
  %723 = load ptr, ptr %722, align 8, !alias.scope !36580, !noalias !36584, !nonnull !1740, !noundef !1740
  %724 = mul i64 %719, 3
  %725 = add i64 %724, -3
  %726 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %727 = load i64, ptr %726, align 8, !noalias !36589, !noundef !1740
  %728 = tail call i64 @llvm.umin.i64(i64 %725, i64 9223372036854775807)
  %729 = tail call i64 @llvm.ssub.sat.i64(i64 %727, i64 %728)
  store i64 %729, ptr %726, align 8, !noalias !36589
  %730 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %731 = load i64, ptr %730, align 8, !noalias !36589, !noundef !1740
  %732 = icmp slt i64 %729, %731
  br i1 %732, label %733, label %.preheader262

733:                                              ; preds = %721
  store i64 %729, ptr %730, align 8, !noalias !36589
  br label %.preheader262

.preheader262:                                    ; preds = %733, %721
  br label %734

734:                                              ; preds = %.preheader262, %737
  %735 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36589
  %736 = icmp slt i64 %735, 0
  br i1 %736, label %737, label %__rustc::__rust_dealloc (.exit43)

737:                                              ; preds = %734
  %738 = add nsw i64 %735, 1
  %739 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %735, i64 %738 acq_rel acquire, align 8, !noalias !36589
  %740 = extractvalue { i64, i1 } %739, 1
  br i1 %740, label %741, label %734

741:                                              ; preds = %737
  %742 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %728 monotonic, align 8, !noalias !36589
  %743 = tail call i64 @llvm.ssub.sat.i64(i64 %742, i64 %728)
  %744 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36589
  br label %745

745:                                              ; preds = %748, %741
  %746 = phi i64 [ %744, %741 ], [ %751, %748 ]
  %747 = icmp slt i64 %743, %746
  br i1 %747, label %748, label %752

748:                                              ; preds = %745
  %749 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %746, i64 %743 monotonic monotonic, align 8, !noalias !36589
  %750 = extractvalue { i64, i1 } %749, 1
  %751 = extractvalue { i64, i1 } %749, 0
  br i1 %750, label %752, label %745

752:                                              ; preds = %748, %745
  %753 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36589
  br label %__rustc::__rust_dealloc (.exit43)

__rustc::__rust_dealloc (.exit43): ; preds = %734, %752
  %754 = icmp ne i64 %725, 0
  tail call void @llvm.assume(i1 %754), !noalias !36589
  tail call void @free(ptr noundef nonnull %723) #93, !noalias !36589
  br label %755

755:                                              ; preds = %__rustc::__rust_dealloc (.exit43), %717
  %756 = getelementptr inbounds nuw i8, ptr %25, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36592)
  %757 = load ptr, ptr %756, align 8, !alias.scope !36595, !noalias !36584, !noundef !1740
  %758 = icmp eq ptr %757, null
  br i1 %758, label %763, label %759

759:                                              ; preds = %755
  %760 = atomicrmw sub ptr %757, i64 1 release, align 8, !noalias !36596
  %761 = icmp eq i64 %760, 1
  br i1 %761, label %762, label %763

762:                                              ; preds = %759
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %756) #92
  br label %763

763:                                              ; preds = %762, %759, %755
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  call void @llvm.lifetime.end.p0(ptr nonnull %25)
  %764 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %764, ptr noundef nonnull align 8 dereferenceable(96) %26, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  br label %705

765:                                              ; preds = %767, %269
  %766 = phi { ptr, i32 } [ %371, %269 ], [ %768, %767 ]
  resume { ptr, i32 } %766

767:                                              ; preds = %270, %269, %259, %234, %218, %88, %84
  %768 = phi { ptr, i32 } [ %271, %270 ], [ %371, %269 ], [ %85, %84 ], [ %85, %88 ], [ %209, %218 ], [ %246, %234 ], [ %260, %259 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %27) #90
          to label %765 unwind label %706
}
