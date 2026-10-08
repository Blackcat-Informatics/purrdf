define void @purrdf_sparql_eval::modifier::eval_group_with::<purrdf_core::ir::dataset::RdfDataset, ()>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %3, i64 noundef range(i64 0, 576460752303423488) %4, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %5, i64 noundef range(i64 0, 76861433640456466) %6, ptr noalias nofree noundef align 16 dereferenceable(1248) %7) unnamed_addr #ATTR personality ptr @rust_eh_personality !guid !ID {
  %9 = alloca [0 x i8], align 1
  %10 = alloca [216 x i8], align 8
  %11 = alloca [24 x i8], align 8
  %12 = alloca [32 x i8], align 8
  %13 = alloca [8 x i8], align 8
  %14 = alloca [16 x i8], align 8
  %15 = alloca [32 x i8], align 8
  %16 = alloca [24 x i8], align 8
  %17 = alloca [48 x i8], align 8
  %18 = alloca [8 x i8], align 8
  %19 = alloca [24 x i8], align 8
  %20 = alloca [24 x i8], align 8
  %21 = alloca [24 x i8], align 8
  %22 = alloca [64 x i8], align 8
  %23 = alloca [32 x i8], align 8
  %24 = alloca [8 x i8], align 8
  %25 = alloca [24 x i8], align 8
  %26 = alloca [24 x i8], align 8
  %27 = alloca [48 x i8], align 8
  %28 = alloca [32 x i8], align 8
  %29 = alloca [8 x i8], align 8
  %30 = alloca [24 x i8], align 8
  %31 = alloca [8 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [24 x i8], align 8
  %34 = alloca [32 x i8], align 8
  %35 = alloca [8 x i8], align 8
  %36 = alloca [16 x i8], align 8
  %37 = alloca [24 x i8], align 8
  %38 = alloca [24 x i8], align 8
  %39 = alloca [23 x i8], align 1
  %40 = alloca [24 x i8], align 8
  %41 = alloca [24 x i8], align 8
  %42 = alloca [24 x i8], align 8
  %43 = alloca [24 x i8], align 8
  %44 = alloca [24 x i8], align 8
  %45 = alloca [32 x i8], align 8
  %46 = alloca [8 x i8], align 8
  %47 = alloca [8 x i8], align 8
  %48 = alloca [48 x i8], align 8
  %49 = alloca [72 x i8], align 8
  %50 = alloca [80 x i8], align 8
  %51 = alloca [72 x i8], align 8
  %52 = alloca [80 x i8], align 8
  %53 = alloca [72 x i8], align 8
  %54 = alloca [80 x i8], align 8
  %55 = alloca [72 x i8], align 8
  %56 = alloca [80 x i8], align 8
  %57 = alloca [72 x i8], align 8
  %58 = alloca [80 x i8], align 8
  %59 = alloca [16 x i8], align 8
  %60 = alloca [40 x i8], align 8
  %61 = alloca [72 x i8], align 8
  %62 = alloca [80 x i8], align 8
  %63 = alloca [48 x i8], align 8
  %64 = alloca [96 x i8], align 16
  %65 = alloca [24 x i8], align 8
  %66 = alloca [24 x i8], align 8
  %67 = alloca [24 x i8], align 8
  %68 = alloca [24 x i8], align 8
  %69 = alloca [24 x i8], align 8
  %70 = alloca [24 x i8], align 8
  %71 = alloca [32 x i8], align 8
  %72 = alloca [160 x i8], align 8
  %73 = alloca [152 x i8], align 8
  %74 = alloca [32 x i8], align 8
  %75 = alloca [8 x i8], align 8
  %76 = alloca [48 x i8], align 8
  %77 = alloca [96 x i8], align 16
  %78 = alloca [32 x i8], align 8
  %79 = alloca [32 x i8], align 8
  %80 = alloca [24 x i8], align 8
  %81 = alloca [96 x i8], align 16
  %82 = alloca [24 x i8], align 8
  %83 = alloca [96 x i8], align 16
  %84 = alloca [32 x i8], align 8
  %85 = alloca [96 x i8], align 16
  %86 = alloca [24 x i8], align 8
  %87 = alloca [160 x i8], align 8
  %88 = alloca [192 x i8], align 8
  %89 = alloca [24 x i8], align 8
  %90 = alloca [24 x i8], align 8
  %91 = alloca [32 x i8], align 8
  %92 = alloca [48 x i8], align 16
  %93 = alloca [24 x i8], align 8
  %94 = alloca [48 x i8], align 16
  %95 = alloca [8 x i8], align 8
  %96 = alloca [8 x i8], align 8
  %97 = alloca [72 x i8], align 8
  %98 = alloca [16 x i8], align 8
  %99 = alloca [64 x i8], align 8
  %100 = alloca [24 x i8], align 8
  %101 = alloca [24 x i8], align 8
  %102 = alloca [40 x i8], align 8
  %103 = alloca [56 x i8], align 8
  %104 = alloca [16 x i8], align 8
  %105 = alloca [56 x i8], align 8
  %106 = alloca [112 x i8], align 16
  %107 = alloca [72 x i8], align 8
  %108 = alloca [32 x i8], align 8
  %109 = alloca [104 x i8], align 8
  %110 = alloca [96 x i8], align 8
  %111 = alloca [96 x i8], align 8
  %112 = alloca [24 x i8], align 8
  %113 = alloca [24 x i8], align 8
  %114 = alloca [96 x i8], align 16
  %115 = alloca [40 x i8], align 8
  %116 = alloca [24 x i8], align 8
  %117 = alloca [40 x i8], align 8
  %118 = alloca [96 x i8], align 16
  %119 = alloca [32 x i8], align 8
  %120 = alloca [176 x i8], align 8
  %121 = alloca [24 x i8], align 8
  %122 = alloca [48 x i8], align 16
  %123 = alloca [176 x i8], align 8
  %124 = alloca [24 x i8], align 8
  %125 = alloca [96 x i8], align 16
  %126 = alloca [24 x i8], align 8
  %127 = alloca [24 x i8], align 8
  %128 = alloca [16 x i8], align 8
  %129 = alloca [24 x i8], align 8
  %130 = alloca [216 x i8], align 8
  %131 = alloca [208 x i8], align 8
  %132 = alloca [32 x i8], align 8
  %133 = alloca [64 x i8], align 8
  %134 = alloca [32 x i8], align 8
  %135 = alloca [224 x i8], align 16
  %136 = alloca [176 x i8], align 16
  %137 = alloca [8 x i8], align 8
  %138 = alloca [168 x i8], align 8
  %139 = alloca [8 x i8], align 8
  %140 = alloca [24 x i8], align 8
  %141 = alloca [24 x i8], align 8
  %142 = alloca [8 x i8], align 8
  %143 = alloca [64 x i8], align 8
  %144 = alloca [24 x i8], align 8
  %145 = alloca [32 x i8], align 8
  %146 = alloca [40 x i8], align 8
  %147 = alloca [32 x i8], align 8
  %148 = alloca [40 x i8], align 8
  %149 = alloca [32 x i8], align 8
  %150 = alloca [8 x i8], align 8
  %151 = alloca [104 x i8], align 8
  %152 = alloca [32 x i8], align 8
  %153 = alloca [32 x i8], align 8
  %154 = alloca [32 x i8], align 8
  %155 = alloca [104 x i8], align 8
  %156 = alloca [96 x i8], align 8
  %157 = alloca [8 x i8], align 8
  %158 = alloca [8 x i8], align 8
  %159 = alloca [56 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %159)
  %.idx = shl nuw nsw i64 %4, 4
  %160 = getelementptr inbounds nuw i8, ptr %3, i64 %.idx
  call void @llvm.lifetime.start.p0(ptr nonnull %103), !noalias !ID
  store i64 0, ptr %103, align 8, !noalias !ID
  %161 = getelementptr inbounds nuw i8, ptr %103, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %161, align 8, !noalias !ID
  %162 = getelementptr inbounds nuw i8, ptr %103, i64 16
  store i64 0, ptr %162, align 8, !noalias !ID
  %163 = getelementptr inbounds nuw i8, ptr %103, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %163, ptr noundef nonnull align 8 dereferenceable(32) @anon.HASH.29.llvm.ID, i64 32, i1 false), !noalias !ID
  %164 = icmp eq i64 %4, 0
  br i1 %164, label %._crit_edge2426, label %.lr.ph

165:                                              ; preds = %177
  %166 = getelementptr inbounds nuw i8, ptr %168, i64 16
  %167 = icmp eq ptr %166, %160
  br i1 %167, label %._crit_edge2426, label %.lr.ph

.lr.ph:                                           ; preds = %8, %165
  %168 = phi ptr [ %166, %165 ], [ %3, %8 ]
  %169 = load ptr, ptr %168, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %170 = getelementptr i8, ptr %168, i64 8
  %171 = load i64, ptr %170, align 8, !noalias !ID
  %172 = atomicrmw add ptr %169, i64 1 monotonic, align 8, !noalias !ID
  %173 = icmp slt i64 %172, 0
  br i1 %173, label %174, label %177

174:                                              ; preds = %.lr.ph
  call void @llvm.trap()
  unreachable

175:                                              ; preds = %177
  %176 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.ID)(ptr noalias nofree noundef align 8 dereferenceable(56) %103) #ATTR
          to label %181 unwind label %179, !noalias !ID

177:                                              ; preds = %.lr.ph
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %178 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %103, ptr noundef nonnull %169, i64 noundef %171)
          to label %165 unwind label %175, !noalias !ID

179:                                              ; preds = %175
  %180 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

181:                                              ; preds = %4025, %4020, %4017, %295, %232, %175
  %182 = phi { ptr, i32 } [ %176, %175 ], [ %233, %232 ], [ %4026, %4025 ], [ %297, %4017 ], [ %297, %295 ], [ %297, %4020 ]
  resume { ptr, i32 } %182

._crit_edge2426:                                  ; preds = %165, %8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %159, ptr noundef nonnull align 8 dereferenceable(56) %103, i64 56, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %103), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %158)
  %183 = getelementptr inbounds nuw i8, ptr %159, i64 16
  %184 = load i64, ptr %183, align 8, !noundef !ID
  %185 = icmp ult i64 %184, 576460752303423488
  call void @llvm.assume(i1 %185)
  store i64 %184, ptr %158, align 8
  %186 = mul nuw nsw i64 %6, 120
  %187 = getelementptr inbounds nuw i8, ptr %5, i64 %186
  %188 = icmp eq i64 %6, 0
  br i1 %188, label %._crit_edge2429, label %.lr.ph2428

189:                                              ; preds = %248
  %190 = icmp eq ptr %237, %187
  br i1 %190, label %._crit_edge2429, label %.lr.ph2428

._crit_edge2429:                                  ; preds = %189, %._crit_edge2426
  call void @llvm.lifetime.start.p0(ptr nonnull %157)
  %191 = getelementptr inbounds nuw i8, ptr %107, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %107)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %191, ptr noundef nonnull align 8 dereferenceable(56) %159, i64 56, i1 false)
  store i64 1, ptr %107, align 8
  %192 = getelementptr inbounds nuw i8, ptr %107, i64 8
  store i64 1, ptr %192, align 8
  %193 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #ATTR, !noalias !ID
  %194 = icmp eq ptr %193, null
  br i1 %194, label %__rustc::__rust_alloc (.exit.thread), label %195

195:                                              ; preds = %._crit_edge2429
  %196 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %197 = load i64, ptr %196, align 8, !noalias !ID, !noundef !ID
  %198 = call i64 @llvm.uadd.sat.i64(i64 %197, i64 1)
  store i64 %198, ptr %196, align 8, !noalias !ID
  %199 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %200 = load i64, ptr %199, align 8, !noalias !ID, !noundef !ID
  %201 = call i64 @llvm.uadd.sat.i64(i64 %200, i64 72)
  store i64 %201, ptr %199, align 8, !noalias !ID
  %202 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %203 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %204 = call i64 @llvm.sadd.sat.i64(i64 %203, i64 72)
  store i64 %204, ptr %202, align 8, !noalias !ID
  %205 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %206 = load i64, ptr %205, align 8, !noalias !ID, !noundef !ID
  %207 = icmp sgt i64 %204, %206
  br i1 %207, label %208, label %.preheader2995

208:                                              ; preds = %195
  store i64 %204, ptr %205, align 8, !noalias !ID
  br label %.preheader2995

.preheader2995:                                   ; preds = %208, %195
  br label %209

209:                                              ; preds = %.preheader2995, %212
  %210 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %211 = icmp slt i64 %210, 0
  br i1 %211, label %212, label %__rustc::__rust_alloc (.exit)

212:                                              ; preds = %209
  %213 = add nsw i64 %210, 1
  %214 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %210, i64 %213 acq_rel acquire, align 8, !noalias !ID
  %215 = extractvalue { i64, i1 } %214, 1
  br i1 %215, label %216, label %209

216:                                              ; preds = %212
  %217 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !ID
  %218 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !ID
  %219 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !ID
  %220 = call i64 @llvm.sadd.sat.i64(i64 %219, i64 72)
  %221 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !ID
  br label %222

222:                                              ; preds = %225, %216
  %223 = phi i64 [ %221, %216 ], [ %228, %225 ]
  %224 = icmp sgt i64 %220, %223
  br i1 %224, label %225, label %229

225:                                              ; preds = %222
  %226 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %223, i64 %220 monotonic monotonic, align 8, !noalias !ID
  %227 = extractvalue { i64, i1 } %226, 1
  %228 = extractvalue { i64, i1 } %226, 0
  br i1 %227, label %229, label %222

229:                                              ; preds = %225, %222
  %230 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %._crit_edge2429
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #ATTR
          to label %231 unwind label %232

231:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

232:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  %233 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.ID)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %191)
          to label %181 unwind label %234

234:                                              ; preds = %232
  %235 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR
  unreachable

.lr.ph2428:                                       ; preds = %._crit_edge2426, %189
  %236 = phi ptr [ %237, %189 ], [ %5, %._crit_edge2426 ]
  %237 = getelementptr inbounds nuw i8, ptr %236, i64 120
  %238 = load i64, ptr %183, align 8, !noundef !ID
  %239 = icmp ult i64 %238, 576460752303423488
  call void @llvm.assume(i1 %239)
  %240 = load ptr, ptr %236, align 8, !nonnull !ID, !noundef !ID
  %241 = atomicrmw add ptr %240, i64 1 monotonic, align 8
  %242 = icmp slt i64 %241, 0
  br i1 %242, label %247, label %243

243:                                              ; preds = %.lr.ph2428
  %244 = getelementptr inbounds nuw i8, ptr %236, i64 8
  %245 = load i64, ptr %244, align 8, !noundef !ID
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %246 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %159, ptr noundef nonnull %240, i64 noundef %245)
          to label %248 unwind label %4021

247:                                              ; preds = %.lr.ph2428
  call void @llvm.trap()
  unreachable

248:                                              ; preds = %243
  %249 = icmp eq i64 %246, %238
  br i1 %249, label %189, label %250

250:                                              ; preds = %248
  %251 = call noundef dereferenceable_or_null(51) ptr @malloc(i64 noundef range(i64 1, 0) 51) #ATTR, !noalias !ID
  %252 = icmp eq ptr %251, null
  br i1 %252, label %__rustc::__rust_alloc (.exit288.thread), label %253

253:                                              ; preds = %250
  %254 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %255 = load i64, ptr %254, align 8, !noalias !ID, !noundef !ID
  %256 = call i64 @llvm.uadd.sat.i64(i64 %255, i64 1)
  store i64 %256, ptr %254, align 8, !noalias !ID
  %257 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %258 = load i64, ptr %257, align 8, !noalias !ID, !noundef !ID
  %259 = call i64 @llvm.uadd.sat.i64(i64 %258, i64 51)
  store i64 %259, ptr %257, align 8, !noalias !ID
  %260 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %261 = load i64, ptr %260, align 8, !noalias !ID, !noundef !ID
  %262 = call i64 @llvm.sadd.sat.i64(i64 %261, i64 51)
  store i64 %262, ptr %260, align 8, !noalias !ID
  %263 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %264 = load i64, ptr %263, align 8, !noalias !ID, !noundef !ID
  %265 = icmp sgt i64 %262, %264
  br i1 %265, label %266, label %.preheader2996

266:                                              ; preds = %253
  store i64 %262, ptr %263, align 8, !noalias !ID
  br label %.preheader2996

.preheader2996:                                   ; preds = %266, %253
  br label %267

267:                                              ; preds = %.preheader2996, %270
  %268 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %269 = icmp slt i64 %268, 0
  br i1 %269, label %270, label %__rustc::__rust_alloc (.exit288)

270:                                              ; preds = %267
  %271 = add nsw i64 %268, 1
  %272 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %268, i64 %271 acq_rel acquire, align 8, !noalias !ID
  %273 = extractvalue { i64, i1 } %272, 1
  br i1 %273, label %274, label %267

274:                                              ; preds = %270
  %275 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !ID
  %276 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 51 monotonic, align 8, !noalias !ID
  %277 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 51 monotonic, align 8, !noalias !ID
  %278 = call i64 @llvm.sadd.sat.i64(i64 %277, i64 51)
  %279 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !ID
  br label %280

280:                                              ; preds = %283, %274
  %281 = phi i64 [ %279, %274 ], [ %286, %283 ]
  %282 = icmp sgt i64 %278, %281
  br i1 %282, label %283, label %287

283:                                              ; preds = %280
  %284 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %281, i64 %278 monotonic monotonic, align 8, !noalias !ID
  %285 = extractvalue { i64, i1 } %284, 1
  %286 = extractvalue { i64, i1 } %284, 0
  br i1 %285, label %287, label %280

287:                                              ; preds = %283, %280
  %288 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_alloc (.exit288)

__rustc::__rust_alloc (.exit288.thread): ; preds = %250
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 1, i64 range(i64 0, -9223372036854775808) 51) #ATTR
          to label %289 unwind label %4023

289:                                              ; preds = %__rustc::__rust_alloc (.exit288.thread)
  unreachable

__rustc::__rust_alloc (.exit288): ; preds = %267, %287
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(51) %251, ptr noundef nonnull readonly align 1 dereferenceable(51) @anon.HASH.528, i64 range(i64 0, -9223372036854775808) 51, i1 false), !noalias !ID
  %290 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 -9223372036854775788, ptr %290, align 16
  %291 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 51, ptr %291, align 8
  %292 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store ptr %251, ptr %292, align 16
  %293 = getelementptr inbounds nuw i8, ptr %0, i64 40
  store i64 51, ptr %293, align 8
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %158)
; call core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.ID)(ptr noalias nofree noundef align 8 dereferenceable(56) %159)
  br label %3526

294:                                              ; preds = %3771, %3768, %3766
  call void @llvm.lifetime.end.p0(ptr nonnull %157)
  call void @llvm.lifetime.end.p0(ptr nonnull %158)
  br label %3526

__rustc::__rust_alloc (.exit):  ; preds = %209, %229
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %193, ptr noundef nonnull align 8 dereferenceable(72) %107, i64 72, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %107)
  store ptr %193, ptr %157, align 8
; invoke <purrdf_sparql_eval::governor::lift::Lift>::at
  invoke void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %109, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
          to label %302 unwind label %299

295:                                              ; preds = %4016, %4013, %4009, %303, %299
  %296 = phi i8 [ %300, %299 ], [ %3519, %303 ], [ %3993, %4016 ], [ %3993, %4009 ], [ %3993, %4013 ]
  %297 = phi { ptr, i32 } [ %301, %299 ], [ %3520, %303 ], [ %3992, %4016 ], [ %3992, %4009 ], [ %3992, %4013 ]
  %298 = trunc nuw i8 %296 to i1
  br i1 %298, label %4017, label %181

299:                                              ; preds = %3765, %390, %__rustc::__rust_alloc (.exit)
  %300 = phi i8 [ 1, %390 ], [ %3339, %3765 ], [ 1, %__rustc::__rust_alloc (.exit) ]
  %301 = landingpad { ptr, i32 }
          cleanup
  br label %295

302:                                              ; preds = %__rustc::__rust_alloc (.exit)
  call void @llvm.lifetime.start.p0(ptr nonnull %152)
  call void @llvm.lifetime.start.p0(ptr nonnull %151)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %106, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %7, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %307 unwind label %304, !inline_history !ID

303:                                              ; preds = %3517
  br i1 %3518, label %3991, label %295

304:                                              ; preds = %3686, %391, %302
  %305 = phi i8 [ 1, %302 ], [ 1, %391 ], [ %3339, %3686 ]
  %306 = landingpad { ptr, i32 }
          cleanup
  br label %3991

307:                                              ; preds = %302
  %308 = load i64, ptr %106, align 16, !range !ID, !noundef !ID
  %309 = trunc nuw i64 %308 to i1
  br i1 %309, label %310, label %391

310:                                              ; preds = %307
  %311 = getelementptr inbounds nuw i8, ptr %106, i64 16
  %312 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %312, ptr noundef nonnull align 16 dereferenceable(96) %311, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %151)
  call void @llvm.lifetime.end.p0(ptr nonnull %152)
  %313 = getelementptr inbounds nuw i8, ptr %109, i64 72
  %314 = load i64, ptr %313, align 8, !range !ID, !noundef !ID
  %315 = icmp ugt i64 %314, 5
  br i1 %315, label %316, label %349

316:                                              ; preds = %310
  %317 = getelementptr inbounds nuw i8, ptr %109, i64 80
  %318 = load ptr, ptr %317, align 8, !nonnull !ID, !noundef !ID
  %319 = mul i64 %314, 3
  %320 = add i64 %319, -3
  %321 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %322 = call i64 @llvm.umin.i64(i64 %320, i64 9223372036854775807)
  %323 = call i64 @llvm.ssub.sat.i64(i64 %321, i64 %322)
  store i64 %323, ptr %202, align 8, !noalias !ID
  %324 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %325 = load i64, ptr %324, align 8, !noalias !ID, !noundef !ID
  %326 = icmp slt i64 %323, %325
  br i1 %326, label %327, label %.preheader2543

327:                                              ; preds = %316
  store i64 %323, ptr %324, align 8, !noalias !ID
  br label %.preheader2543

.preheader2543:                                   ; preds = %327, %316
  br label %328

328:                                              ; preds = %.preheader2543, %331
  %329 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %330 = icmp slt i64 %329, 0
  br i1 %330, label %331, label %__rustc::__rust_dealloc (.exit)

331:                                              ; preds = %328
  %332 = add nsw i64 %329, 1
  %333 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %329, i64 %332 acq_rel acquire, align 8, !noalias !ID
  %334 = extractvalue { i64, i1 } %333, 1
  br i1 %334, label %335, label %328

335:                                              ; preds = %331
  %336 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %322 monotonic, align 8, !noalias !ID
  %337 = call i64 @llvm.ssub.sat.i64(i64 %336, i64 %322)
  %338 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %339

339:                                              ; preds = %342, %335
  %340 = phi i64 [ %338, %335 ], [ %345, %342 ]
  %341 = icmp slt i64 %337, %340
  br i1 %341, label %342, label %346

342:                                              ; preds = %339
  %343 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %340, i64 %337 monotonic monotonic, align 8, !noalias !ID
  %344 = extractvalue { i64, i1 } %343, 1
  %345 = extractvalue { i64, i1 } %343, 0
  br i1 %344, label %346, label %339

346:                                              ; preds = %342, %339
  %347 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %328, %346
  %348 = icmp ne i64 %320, 0
  call void @llvm.assume(i1 %348), !noalias !ID
  call void @free(ptr noundef nonnull %318) #ATTR, !noalias !ID
  br label %349

349:                                              ; preds = %__rustc::__rust_dealloc (.exit), %310
  %350 = load i64, ptr %109, align 8, !range !ID, !noundef !ID
  %351 = icmp sgt i64 %350, 0
  br i1 %351, label %352, label %383

352:                                              ; preds = %349
  %353 = getelementptr inbounds nuw i8, ptr %109, i64 8
  %354 = load ptr, ptr %353, align 8, !nonnull !ID, !noundef !ID
  %355 = mul nuw i64 %350, 3
  %356 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %357 = call i64 @llvm.umin.i64(i64 %355, i64 9223372036854775807)
  %358 = call i64 @llvm.ssub.sat.i64(i64 %356, i64 %357)
  store i64 %358, ptr %202, align 8, !noalias !ID
  %359 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %360 = load i64, ptr %359, align 8, !noalias !ID, !noundef !ID
  %361 = icmp slt i64 %358, %360
  br i1 %361, label %362, label %.preheader2542

362:                                              ; preds = %352
  store i64 %358, ptr %359, align 8, !noalias !ID
  br label %.preheader2542

.preheader2542:                                   ; preds = %362, %352
  br label %363

363:                                              ; preds = %.preheader2542, %366
  %364 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %365 = icmp slt i64 %364, 0
  br i1 %365, label %366, label %__rustc::__rust_dealloc (.exit289)

366:                                              ; preds = %363
  %367 = add nsw i64 %364, 1
  %368 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %364, i64 %367 acq_rel acquire, align 8, !noalias !ID
  %369 = extractvalue { i64, i1 } %368, 1
  br i1 %369, label %370, label %363

370:                                              ; preds = %366
  %371 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %357 monotonic, align 8, !noalias !ID
  %372 = call i64 @llvm.ssub.sat.i64(i64 %371, i64 %357)
  %373 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %374

374:                                              ; preds = %377, %370
  %375 = phi i64 [ %373, %370 ], [ %380, %377 ]
  %376 = icmp slt i64 %372, %375
  br i1 %376, label %377, label %381

377:                                              ; preds = %374
  %378 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %375, i64 %372 monotonic monotonic, align 8, !noalias !ID
  %379 = extractvalue { i64, i1 } %378, 1
  %380 = extractvalue { i64, i1 } %378, 0
  br i1 %379, label %381, label %374

381:                                              ; preds = %377, %374
  %382 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit289)

__rustc::__rust_dealloc (.exit289): ; preds = %363, %381
  call void @free(ptr noundef nonnull %354) #ATTR, !noalias !ID
  br label %383

383:                                              ; preds = %__rustc::__rust_dealloc (.exit289), %349
  %384 = getelementptr inbounds nuw i8, ptr %109, i64 96
  %385 = load ptr, ptr %384, align 8, !noundef !ID
  %386 = icmp eq ptr %385, null
  br i1 %386, label %3768, label %387

387:                                              ; preds = %383
  %388 = atomicrmw sub ptr %385, i64 1 release, align 8, !noalias !ID
  %389 = icmp eq i64 %388, 1
  br i1 %389, label %390, label %3768

390:                                              ; preds = %387
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %384) #ATTR
          to label %3768 unwind label %299

391:                                              ; preds = %307
  %392 = getelementptr inbounds nuw i8, ptr %106, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %151, ptr noundef nonnull align 8 dereferenceable(96) %392, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %152, ptr noalias nofree noundef align 8 dereferenceable(104) %109, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %151)
          to label %393 unwind label %304

393:                                              ; preds = %391
  %394 = load i64, ptr %152, align 8, !range !ID, !noundef !ID
  %395 = icmp eq i64 %394, -1
  br i1 %395, label %3935, label %396

396:                                              ; preds = %393
  call void @llvm.lifetime.start.p0(ptr nonnull %153)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %153, ptr noundef nonnull align 8 dereferenceable(32) %152, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %151)
  call void @llvm.lifetime.end.p0(ptr nonnull %152)
  call void @llvm.lifetime.start.p0(ptr nonnull %150)
  %397 = getelementptr inbounds nuw i8, ptr %153, i64 24
  %398 = load ptr, ptr %397, align 8, !nonnull !ID, !noundef !ID
  %399 = atomicrmw add ptr %398, i64 1 monotonic, align 8
  %400 = icmp slt i64 %399, 0
  br i1 %400, label %401, label %412

401:                                              ; preds = %396
  call void @llvm.trap()
  unreachable

402:                                              ; preds = %__rustc::__rust_dealloc (.exit311), %3901, %473, %410
  %403 = phi i1 [ true, %410 ], [ true, %473 ], [ %3904, %3901 ], [ %3904, %__rustc::__rust_dealloc (.exit311) ]
  %404 = phi i8 [ 1, %410 ], [ 1, %473 ], [ %3903, %3901 ], [ %3903, %__rustc::__rust_dealloc (.exit311) ]
  %405 = phi { ptr, i32 } [ %411, %410 ], [ %474, %473 ], [ %3902, %3901 ], [ %3902, %__rustc::__rust_dealloc (.exit311) ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %406 = load ptr, ptr %150, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %407 = atomicrmw sub ptr %406, i64 1 release, align 8, !noalias !ID
  %408 = icmp eq i64 %407, 1
  br i1 %408, label %409, label %3517

409:                                              ; preds = %402
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %150) #ATTR
          to label %3517 unwind label %1290

410:                                              ; preds = %__rustc::__rust_alloc (.exit290.thread), %420
  %411 = landingpad { ptr, i32 }
          cleanup
  br label %402

412:                                              ; preds = %396
  %413 = load ptr, ptr %397, align 8, !nonnull !ID, !noundef !ID
  store ptr %413, ptr %150, align 8
  %414 = getelementptr i8, ptr %193, i64 24
  %415 = load ptr, ptr %414, align 8, !nonnull !ID, !noundef !ID
  %416 = getelementptr i8, ptr %193, i64 32
  %417 = load i64, ptr %416, align 8, !noundef !ID
  %418 = load i64, ptr %158, align 8, !noundef !ID
  %419 = icmp ugt i64 %418, %417
  br i1 %419, label %420, label %421, !prof !ID

420:                                              ; preds = %412
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef 0, i64 noundef %418, i64 noundef %417, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.536) #ATTR
          to label %1111 unwind label %410

421:                                              ; preds = %412
  %422 = shl nuw i64 %418, 4
  %423 = icmp eq i64 %418, 0
  br i1 %423, label %.loopexit386, label %424

424:                                              ; preds = %421
  %425 = call noundef ptr @malloc(i64 noundef range(i64 1, 0) %422) #ATTR, !noalias !ID
  %426 = icmp eq ptr %425, null
  br i1 %426, label %__rustc::__rust_alloc (.exit290.thread), label %427

427:                                              ; preds = %424
  %428 = load i64, ptr %196, align 8, !noalias !ID, !noundef !ID
  %429 = call i64 @llvm.uadd.sat.i64(i64 %428, i64 1)
  store i64 %429, ptr %196, align 8, !noalias !ID
  %430 = load i64, ptr %199, align 8, !noalias !ID, !noundef !ID
  %431 = call i64 @llvm.uadd.sat.i64(i64 %430, i64 %422)
  store i64 %431, ptr %199, align 8, !noalias !ID
  %432 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %433 = call i64 @llvm.umin.i64(i64 %422, i64 9223372036854775807)
  %434 = call i64 @llvm.sadd.sat.i64(i64 %432, i64 %433)
  store i64 %434, ptr %202, align 8, !noalias !ID
  %435 = load i64, ptr %205, align 8, !noalias !ID, !noundef !ID
  %436 = icmp sgt i64 %434, %435
  br i1 %436, label %437, label %.preheader2994

437:                                              ; preds = %427
  store i64 %434, ptr %205, align 8, !noalias !ID
  br label %.preheader2994

.preheader2994:                                   ; preds = %437, %427
  br label %438

438:                                              ; preds = %.preheader2994, %441
  %439 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %440 = icmp slt i64 %439, 0
  br i1 %440, label %441, label %__rustc::__rust_alloc (.exit290.preheader)

441:                                              ; preds = %438
  %442 = add nsw i64 %439, 1
  %443 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %439, i64 %442 acq_rel acquire, align 8, !noalias !ID
  %444 = extractvalue { i64, i1 } %443, 1
  br i1 %444, label %445, label %438

445:                                              ; preds = %441
  %446 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !ID
  %447 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %422 monotonic, align 8, !noalias !ID
  %448 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %433 monotonic, align 8, !noalias !ID
  %449 = call i64 @llvm.sadd.sat.i64(i64 %448, i64 %433)
  %450 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !ID
  br label %451

451:                                              ; preds = %454, %445
  %452 = phi i64 [ %450, %445 ], [ %457, %454 ]
  %453 = icmp sgt i64 %449, %452
  br i1 %453, label %454, label %458

454:                                              ; preds = %451
  %455 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %452, i64 %449 monotonic monotonic, align 8, !noalias !ID
  %456 = extractvalue { i64, i1 } %455, 1
  %457 = extractvalue { i64, i1 } %455, 0
  br i1 %456, label %458, label %451

458:                                              ; preds = %454, %451
  %459 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_alloc (.exit290.preheader)

__rustc::__rust_alloc (.exit290.preheader): ; preds = %438, %458
  br label %__rustc::__rust_alloc (.exit290)

__rustc::__rust_alloc (.exit290.thread): ; preds = %424
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %422) #ATTR
          to label %460 unwind label %410

460:                                              ; preds = %__rustc::__rust_alloc (.exit290.thread)
  unreachable

__rustc::__rust_alloc (.exit290): ; preds = %__rustc::__rust_alloc (.exit290.preheader), %466
  %461 = phi i64 [ %471, %466 ], [ 0, %__rustc::__rust_alloc (.exit290.preheader) ]
  %462 = getelementptr inbounds nuw [16 x i8], ptr %415, i64 %461
  %463 = load ptr, ptr %150, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %464 = getelementptr inbounds nuw i8, ptr %463, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema>::index_of
  %465 = invoke fastcc { i64, i64 } @<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.ID)(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %464, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %462) #ATTR
          to label %466 unwind label %473, !noalias !ID

466:                                              ; preds = %__rustc::__rust_alloc (.exit290)
  %467 = extractvalue { i64, i64 } %465, 0
  %468 = extractvalue { i64, i64 } %465, 1
  %469 = getelementptr inbounds nuw [16 x i8], ptr %425, i64 %461
  store i64 %467, ptr %469, align 8, !noalias !ID
  %470 = getelementptr inbounds nuw i8, ptr %469, i64 8
  store i64 %468, ptr %470, align 8, !noalias !ID
  %471 = add nuw i64 %461, 1
  %472 = icmp eq i64 %471, %418
  br i1 %472, label %.loopexit386, label %__rustc::__rust_alloc (.exit290)

473:                                              ; preds = %__rustc::__rust_alloc (.exit290)
  %474 = landingpad { ptr, i32 }
          cleanup
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %425, i64 noundef %422, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %402

.loopexit386:                                     ; preds = %466, %421
  %475 = phi ptr [ inttoptr (i64 8 to ptr), %421 ], [ %425, %466 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %149)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %149, ptr noundef nonnull align 8 dereferenceable(32) @anon.HASH.29.llvm.ID, i64 32, i1 false)
  %476 = getelementptr inbounds nuw i8, ptr %153, i64 8
  %477 = load ptr, ptr %476, align 8, !nonnull !ID, !noundef !ID
  %478 = getelementptr inbounds nuw i8, ptr %153, i64 16
  %479 = load i64, ptr %478, align 8, !noundef !ID
  %480 = mul nuw nsw i64 %479, 40
  %481 = getelementptr inbounds nuw i8, ptr %477, i64 %480
  %482 = icmp eq i64 %479, 0
  br i1 %482, label %633, label %483

483:                                              ; preds = %.loopexit386
  %484 = getelementptr inbounds nuw [16 x i8], ptr %475, i64 %418
  %485 = getelementptr inbounds nuw i8, ptr %102, i64 16
  %486 = icmp ugt i64 %418, 4
  %487 = getelementptr inbounds nuw i8, ptr %102, i64 8
  %488 = getelementptr inbounds nuw i8, ptr %149, i64 24
  %489 = getelementptr inbounds nuw i8, ptr %105, i64 8
  %490 = getelementptr inbounds nuw i8, ptr %105, i64 16
  %491 = getelementptr inbounds nuw i8, ptr %105, i64 24
  %492 = getelementptr inbounds nuw i8, ptr %105, i64 40
  %493 = getelementptr inbounds nuw i8, ptr %105, i64 48
  br label %498

494:                                              ; preds = %3822, %.loopexit385
  %495 = landingpad { ptr, i32 }
          cleanup
  br label %3933

496:                                              ; preds = %659
  %497 = landingpad { ptr, i32 }
          cleanup
  br label %3933

498:                                              ; preds = %3828, %483
  %499 = phi ptr [ %477, %483 ], [ %501, %3828 ]
  %500 = phi i64 [ 0, %483 ], [ %502, %3828 ]
  %501 = getelementptr inbounds nuw i8, ptr %499, i64 40
  %502 = add nuw nsw i64 %500, 1
  call void @llvm.lifetime.start.p0(ptr nonnull %148)
  call void @llvm.lifetime.start.p0(ptr nonnull %102), !noalias !ID
  store i64 1, ptr %102, align 8, !noalias !ID
  br i1 %486, label %503, label %508, !prof !ID

503:                                              ; preds = %498
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %102, i64 noundef 0, i64 noundef %418, i1 noundef zeroext true) #ATTR
          to label %504 unwind label %618, !noalias !ID

504:                                              ; preds = %503
  %505 = load i64, ptr %102, align 8, !range !ID, !alias.scope !ID, !noalias !ID
  %506 = load ptr, ptr %487, align 8, !alias.scope !ID, !noalias !ID
  %507 = add i64 %505, -1
  br label %508

508:                                              ; preds = %504, %498
  %509 = phi ptr [ %506, %504 ], [ undef, %498 ]
  %510 = phi i64 [ %507, %504 ], [ 0, %498 ]
  %511 = icmp ugt i64 %510, 4
  %512 = call i64 @llvm.umax.i64(i64 %510, i64 4)
  %513 = select i1 %511, ptr %509, ptr %487
  %514 = select i1 %511, ptr %485, ptr %102
  %515 = load i64, ptr %514, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %516 = add i64 %515, -1
  %517 = icmp ult i64 %516, %512
  br i1 %517, label %518, label %523

518:                                              ; preds = %508
  %519 = getelementptr inbounds nuw i8, ptr %499, i64 8
  %520 = getelementptr inbounds nuw i8, ptr %499, i64 16
  br label %530

521:                                              ; preds = %606
  %522 = add nuw i64 %512, 1
  br label %523

523:                                              ; preds = %521, %508
  %524 = phi ptr [ %475, %508 ], [ %535, %521 ]
  %525 = phi i64 [ %515, %508 ], [ %522, %521 ]
  store i64 %525, ptr %514, align 8, !noalias !ID
  %526 = icmp eq ptr %524, %484
  br i1 %526, label %.loopexit385, label %527

527:                                              ; preds = %523
  %528 = getelementptr inbounds nuw i8, ptr %499, i64 8
  %529 = getelementptr inbounds nuw i8, ptr %499, i64 16
  br label %557

530:                                              ; preds = %606, %518
  %531 = phi i64 [ %516, %518 ], [ %609, %606 ]
  %532 = phi ptr [ %475, %518 ], [ %535, %606 ]
  %533 = icmp eq ptr %532, %484
  br i1 %533, label %611, label %534

534:                                              ; preds = %530
  %535 = getelementptr inbounds nuw i8, ptr %532, i64 16
  %536 = load i64, ptr %532, align 8, !range !ID, !noalias !ID, !noundef !ID
  %537 = getelementptr i8, ptr %532, i64 8
  %538 = load i64, ptr %537, align 8, !noalias !ID
  %539 = trunc nuw i64 %536 to i1
  br i1 %539, label %540, label %606

540:                                              ; preds = %534
  %541 = load i64, ptr %499, align 8, !range !ID, !noalias !ID, !noundef !ID
  %542 = add i64 %541, -1
  %543 = icmp ugt i64 %542, 4
  br i1 %543, label %544, label %548

544:                                              ; preds = %540
  %545 = load ptr, ptr %519, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %546 = load i64, ptr %520, align 8, !noalias !ID, !noundef !ID
  %547 = add i64 %546, -1
  br label %548

548:                                              ; preds = %544, %540
  %549 = phi i64 [ %547, %544 ], [ %542, %540 ]
  %550 = phi ptr [ %545, %544 ], [ %519, %540 ]
  %551 = icmp ult i64 %538, %549
  br i1 %551, label %554, label %552

552:                                              ; preds = %548
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %538, i64 noundef %549, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.787) #ATTR
          to label %553 unwind label %613, !noalias !ID

553:                                              ; preds = %552
  unreachable

554:                                              ; preds = %548
  %555 = getelementptr inbounds nuw [8 x i8], ptr %550, i64 %538
  %556 = load <2 x i32>, ptr %555, align 4, !noalias !ID
  br label %606

557:                                              ; preds = %600, %527
  %558 = phi ptr [ %524, %527 ], [ %559, %600 ]
  %559 = getelementptr inbounds nuw i8, ptr %558, i64 16
  %560 = load i64, ptr %558, align 8, !range !ID, !noalias !ID, !noundef !ID
  %561 = getelementptr i8, ptr %558, i64 8
  %562 = load i64, ptr %561, align 8, !noalias !ID
  %563 = trunc nuw i64 %560 to i1
  br i1 %563, label %564, label %581

564:                                              ; preds = %557
  %565 = load i64, ptr %499, align 8, !range !ID, !noalias !ID, !noundef !ID
  %566 = add i64 %565, -1
  %567 = icmp ugt i64 %566, 4
  br i1 %567, label %568, label %572

568:                                              ; preds = %564
  %569 = load ptr, ptr %528, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %570 = load i64, ptr %529, align 8, !noalias !ID, !noundef !ID
  %571 = add i64 %570, -1
  br label %572

572:                                              ; preds = %568, %564
  %573 = phi i64 [ %571, %568 ], [ %566, %564 ]
  %574 = phi ptr [ %569, %568 ], [ %528, %564 ]
  %575 = icmp ult i64 %562, %573
  br i1 %575, label %578, label %576

576:                                              ; preds = %572
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %562, i64 noundef %573, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.787) #ATTR
          to label %577 unwind label %620, !noalias !ID

577:                                              ; preds = %576
  unreachable

578:                                              ; preds = %572
  %579 = getelementptr inbounds nuw [8 x i8], ptr %574, i64 %562
  %580 = load <2 x i32>, ptr %579, align 4, !noalias !ID
  br label %581

581:                                              ; preds = %578, %557
  %582 = phi <2 x i32> [ <i32 2, i32 undef>, %557 ], [ %580, %578 ]
  %583 = load i64, ptr %102, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %584 = add i64 %583, -1
  %585 = icmp ugt i64 %584, 4
  %586 = load ptr, ptr %487, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID
  %587 = select i1 %585, ptr %586, ptr %487
  %588 = select i1 %585, ptr %485, ptr %102
  %589 = call i64 @llvm.umax.i64(i64 %584, i64 4)
  %590 = load i64, ptr %588, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %591 = add i64 %590, -1
  %592 = icmp eq i64 %591, %589
  br i1 %592, label %593, label %600, !prof !ID

593:                                              ; preds = %581
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %102, i64 noundef %589, i64 noundef 1, i1 noundef zeroext true) #ATTR
          to label %594 unwind label %616, !noalias !ID

594:                                              ; preds = %593
  %595 = load i64, ptr %102, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %596 = icmp ugt i64 %595, 5
  %597 = load ptr, ptr %487, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID
  %598 = select i1 %596, ptr %597, ptr %487
  %599 = select i1 %596, ptr %485, ptr %102
  br label %600

600:                                              ; preds = %594, %581
  %601 = phi ptr [ %598, %594 ], [ %587, %581 ]
  %602 = phi ptr [ %599, %594 ], [ %588, %581 ]
  %603 = getelementptr inbounds nuw [8 x i8], ptr %601, i64 %591
  store <2 x i32> %582, ptr %603, align 4, !noalias !ID
  %604 = add i64 %590, 1
  store i64 %604, ptr %602, align 8, !alias.scope !ID, !noalias !ID
  %605 = icmp eq ptr %559, %484
  br i1 %605, label %.loopexit385, label %557

606:                                              ; preds = %554, %534
  %607 = phi <2 x i32> [ <i32 2, i32 undef>, %534 ], [ %556, %554 ]
  %608 = getelementptr inbounds nuw [8 x i8], ptr %513, i64 %531
  store <2 x i32> %607, ptr %608, align 4, !noalias !ID
  %609 = add i64 %531, 1
  %610 = icmp eq i64 %609, %512
  br i1 %610, label %521, label %530

611:                                              ; preds = %530
  %612 = add nuw i64 %531, 1
  store i64 %612, ptr %514, align 8, !noalias !ID
  br label %.loopexit385

613:                                              ; preds = %552
  %614 = landingpad { ptr, i32 }
          cleanup
  %615 = add nuw i64 %531, 1
  store i64 %615, ptr %514, align 8, !noalias !ID
  br label %622

616:                                              ; preds = %593
  %617 = landingpad { ptr, i32 }
          cleanup
  br label %622

618:                                              ; preds = %503
  %619 = landingpad { ptr, i32 }
          cleanup
  br label %622

620:                                              ; preds = %576
  %621 = landingpad { ptr, i32 }
          cleanup
  br label %622

622:                                              ; preds = %620, %618, %616, %613
  %623 = phi { ptr, i32 } [ %614, %613 ], [ %617, %616 ], [ %619, %618 ], [ %621, %620 ]
  %624 = load i64, ptr %102, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %625 = icmp ugt i64 %624, 5
  br i1 %625, label %626, label %3933

626:                                              ; preds = %622
  %627 = load ptr, ptr %487, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %628 = shl i64 %624, 3
  %629 = add i64 %628, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %627, i64 noundef %629, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %3933

630:                                              ; preds = %3828
  %631 = load i64, ptr %488, align 8
  %632 = or i64 %631, %4
  br label %633

633:                                              ; preds = %630, %.loopexit386
  %634 = phi i64 [ %631, %630 ], [ 0, %.loopexit386 ]
  %635 = phi i64 [ %632, %630 ], [ %4, %.loopexit386 ]
  %636 = getelementptr inbounds nuw i8, ptr %149, i64 24
  %637 = icmp ne i64 %635, 0
  %638 = icmp eq i64 %6, 0
  %639 = or i1 %638, %637
  br i1 %639, label %640, label %659

640:                                              ; preds = %697, %633
  %641 = phi i64 [ %698, %697 ], [ %634, %633 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %144)
  call void @llvm.lifetime.start.p0(ptr nonnull %143)
  %642 = load ptr, ptr %149, align 8, !nonnull !ID, !noundef !ID
  %643 = getelementptr inbounds nuw i8, ptr %149, i64 8
  %644 = load i64, ptr %643, align 8
  %645 = load <16 x i8>, ptr %642, align 16, !noalias !ID
  %646 = icmp eq i64 %644, 0
  br i1 %646, label %699, label %647

647:                                              ; preds = %640
  %648 = mul i64 %644, 72
  %649 = add i64 %648, 72
  %650 = icmp ult i64 %649, -15
  call void @llvm.assume(i1 %650)
  %651 = and i64 %648, -16
  %652 = add i64 %651, 80
  %653 = add i64 %644, 17
  %654 = add i64 %653, %652
  %655 = icmp uge i64 %654, %652
  call void @llvm.assume(i1 %655)
  %656 = icmp ult i64 %654, 9223372036854775793
  call void @llvm.assume(i1 %656)
  %657 = sub i64 -80, %651
  %658 = getelementptr inbounds i8, ptr %642, i64 %657
  br label %699

659:                                              ; preds = %633
  call void @llvm.lifetime.start.p0(ptr nonnull %147)
  call void @llvm.lifetime.start.p0(ptr nonnull %146)
  store i64 1, ptr %146, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %145)
  %660 = getelementptr inbounds nuw i8, ptr %145, i64 16
  call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %145, i8 0, i64 16, i1 false)
  store ptr inttoptr (i64 8 to ptr), ptr %660, align 8
  %661 = getelementptr inbounds nuw i8, ptr %145, i64 24
  store i64 0, ptr %661, align 8
; invoke <hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::insert
  invoke fastcc void @<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::insert(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %147, ptr noalias nofree noundef align 8 dereferenceable(32) %149, ptr noalias nofree noundef align 8 captures(address) dereferenceable(40) %146, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %145)
          to label %662 unwind label %496

662:                                              ; preds = %659
  call void @llvm.lifetime.end.p0(ptr nonnull %145)
  call void @llvm.lifetime.end.p0(ptr nonnull %146)
  %663 = getelementptr inbounds nuw i8, ptr %147, i64 8
  %664 = load i64, ptr %663, align 8, !range !ID, !noundef !ID
  %665 = icmp sgt i64 %664, 0
  br i1 %665, label %666, label %697

666:                                              ; preds = %662
  %667 = getelementptr inbounds nuw i8, ptr %147, i64 16
  %668 = load ptr, ptr %667, align 8, !nonnull !ID, !noundef !ID
  %669 = shl nuw i64 %664, 3
  %670 = load i64, ptr %202, align 8, !noundef !ID
  %671 = call i64 @llvm.umin.i64(i64 %669, i64 9223372036854775807)
  %672 = call i64 @llvm.ssub.sat.i64(i64 %670, i64 %671)
  store i64 %672, ptr %202, align 8
  %673 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %674 = load i64, ptr %673, align 8, !noundef !ID
  %675 = icmp slt i64 %672, %674
  br i1 %675, label %676, label %.preheader2968

676:                                              ; preds = %666
  store i64 %672, ptr %673, align 8
  br label %.preheader2968

.preheader2968:                                   ; preds = %676, %666
  br label %677

677:                                              ; preds = %.preheader2968, %680
  %678 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8
  %679 = icmp slt i64 %678, 0
  br i1 %679, label %680, label %__rustc::__rust_dealloc (.exit291)

680:                                              ; preds = %677
  %681 = add nsw i64 %678, 1
  %682 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %678, i64 %681 acq_rel acquire, align 8
  %683 = extractvalue { i64, i1 } %682, 1
  br i1 %683, label %684, label %677

684:                                              ; preds = %680
  %685 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %671 monotonic, align 8
  %686 = call i64 @llvm.ssub.sat.i64(i64 %685, i64 %671)
  %687 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %688

688:                                              ; preds = %691, %684
  %689 = phi i64 [ %687, %684 ], [ %694, %691 ]
  %690 = icmp slt i64 %686, %689
  br i1 %690, label %691, label %695

691:                                              ; preds = %688
  %692 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %689, i64 %686 monotonic monotonic, align 8
  %693 = extractvalue { i64, i1 } %692, 1
  %694 = extractvalue { i64, i1 } %692, 0
  br i1 %693, label %695, label %688

695:                                              ; preds = %691, %688
  %696 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit291)

__rustc::__rust_dealloc (.exit291): ; preds = %677, %695
  call void @free(ptr noundef nonnull %668) #ATTR
  br label %697

697:                                              ; preds = %__rustc::__rust_dealloc (.exit291), %662
  call void @llvm.lifetime.end.p0(ptr nonnull %147)
  %698 = load i64, ptr %636, align 8
  br label %640

699:                                              ; preds = %647, %640
  %700 = phi i64 [ undef, %640 ], [ %654, %647 ]
  %701 = phi ptr [ undef, %640 ], [ %658, %647 ]
  %702 = phi i64 [ 0, %640 ], [ 16, %647 ]
  %703 = getelementptr inbounds nuw i8, ptr %642, i64 16
  %704 = icmp sgt <16 x i8> %645, splat (i8 -1)
  %705 = getelementptr i8, ptr %642, i64 %644
  %706 = getelementptr i8, ptr %705, i64 1
  store i64 %702, ptr %143, align 8
  %707 = getelementptr inbounds nuw i8, ptr %143, i64 8
  store i64 %700, ptr %707, align 8
  %708 = getelementptr inbounds nuw i8, ptr %143, i64 16
  store ptr %701, ptr %708, align 8
  %709 = getelementptr inbounds nuw i8, ptr %143, i64 24
  store ptr %642, ptr %709, align 8
  %710 = getelementptr inbounds nuw i8, ptr %143, i64 32
  store ptr %703, ptr %710, align 8
  %711 = getelementptr inbounds nuw i8, ptr %143, i64 40
  store ptr %706, ptr %711, align 8
  %712 = getelementptr inbounds nuw i8, ptr %143, i64 48
  store <16 x i1> %704, ptr %712, align 8
  %713 = getelementptr inbounds nuw i8, ptr %143, i64 56
  store i64 %641, ptr %713, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %100)
  call void @llvm.lifetime.start.p0(ptr nonnull %101), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %714 = icmp eq i64 %641, 0
  br i1 %714, label %740, label %715

715:                                              ; preds = %699
  %716 = bitcast <16 x i1> %704 to i16
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %717 = icmp eq i16 %716, 0
  br i1 %717, label %.preheader382, label %727

718:                                              ; preds = %.preheader382
  store ptr %724, ptr %710, align 8, !alias.scope !ID, !noalias !ID
  store ptr %723, ptr %709, align 8, !alias.scope !ID, !noalias !ID
  br label %727

.preheader382:                                    ; preds = %715, %.preheader382
  %719 = phi ptr [ %724, %.preheader382 ], [ %703, %715 ]
  %720 = phi ptr [ %723, %.preheader382 ], [ %642, %715 ]
  %721 = load <16 x i8>, ptr %719, align 16, !noalias !ID
  %722 = icmp sgt <16 x i8> %721, splat (i8 -1)
  %723 = getelementptr inbounds i8, ptr %720, i64 -1152
  %724 = getelementptr inbounds nuw i8, ptr %719, i64 16
  %725 = bitcast <16 x i1> %722 to i16
  %726 = icmp eq i16 %725, 0
  br i1 %726, label %.preheader382, label %718

727:                                              ; preds = %718, %715
  %728 = phi ptr [ %723, %718 ], [ %642, %715 ]
  %729 = phi i16 [ %725, %718 ], [ %716, %715 ]
  %730 = add i16 %729, -1
  %731 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %729, i1 true)
  %732 = zext nneg i16 %731 to i64
  %733 = and i16 %730, %729
  store i16 %733, ptr %712, align 8, !alias.scope !ID, !noalias !ID
  %734 = sub nsw i64 0, %732
  %735 = getelementptr inbounds [72 x i8], ptr %728, i64 %734
  %736 = add i64 %641, -1
  store i64 %736, ptr %713, align 8, !alias.scope !ID, !noalias !ID
  %737 = getelementptr inbounds i8, ptr %735, i64 -24
  %738 = load i64, ptr %737, align 8, !noalias !ID
  %739 = icmp eq i64 %738, -1
  br i1 %739, label %740, label %754

740:                                              ; preds = %727, %699
  store i64 0, ptr %144, align 8, !alias.scope !ID, !noalias !ID
  %741 = getelementptr inbounds nuw i8, ptr %144, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %741, align 8, !alias.scope !ID, !noalias !ID
  %742 = getelementptr inbounds nuw i8, ptr %144, i64 16
  store i64 0, ptr %742, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %101), !noalias !ID
; call core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
  call fastcc void @core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(64) %143), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %100)
  call void @llvm.lifetime.end.p0(ptr nonnull %143)
  br label %877

743:                                              ; preds = %774
  %744 = landingpad { ptr, i32 }
          cleanup
  %745 = icmp ugt i64 %759, 5
  br i1 %745, label %746, label %749

746:                                              ; preds = %743
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %761) ]
  %747 = shl i64 %759, 3
  %748 = add i64 %747, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %761, i64 noundef %748, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %749

749:                                              ; preds = %746, %743
  %750 = icmp eq i64 %738, 0
  br i1 %750, label %753, label %751

751:                                              ; preds = %749
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %763) ]
  %752 = shl nuw i64 %738, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %763, i64 noundef %752, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %753

753:                                              ; preds = %751, %749
; call core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
  call fastcc void @core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(64) %143), !noalias !ID
  br label %3901

754:                                              ; preds = %727
  %755 = getelementptr inbounds i8, ptr %735, i64 -16
  %756 = getelementptr inbounds i8, ptr %735, i64 -32
  %757 = load i64, ptr %756, align 8, !noalias !ID
  %758 = getelementptr inbounds i8, ptr %735, i64 -72
  %759 = load i64, ptr %758, align 8, !noalias !ID
  %760 = getelementptr inbounds i8, ptr %735, i64 -64
  %761 = load ptr, ptr %760, align 8, !noalias !ID
  %762 = getelementptr inbounds i8, ptr %735, i64 -56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %100, ptr noundef nonnull align 8 dereferenceable(24) %762, i64 24, i1 false), !noalias !ID
  %763 = load ptr, ptr %755, align 8, !noalias !ID
  %764 = getelementptr inbounds i8, ptr %735, i64 -8
  %765 = load i64, ptr %764, align 8, !noalias !ID
  %766 = call i64 @llvm.umax.i64(i64 %641, i64 4)
  %767 = mul i64 %766, 72
  %768 = icmp ugt i64 %641, 128102389400760775
  br i1 %768, label %774, label %769, !prof !ID

769:                                              ; preds = %754
  %770 = icmp eq i64 %767, 0
  br i1 %770, label %777, label %771

771:                                              ; preds = %769
; call __rustc::__rust_alloc
  %772 = call noundef align 8 ptr @__rustc::__rust_alloc(i64 noundef %767, i64 noundef range(i64 1, 17) 8) #ATTR, !noalias !ID
  %773 = icmp eq ptr %772, null
  br i1 %773, label %774, label %777

774:                                              ; preds = %771, %754
  %775 = phi i64 [ 8, %771 ], [ 0, %754 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %775, i64 %767) #ATTR
          to label %776 unwind label %743, !noalias !ID

776:                                              ; preds = %774
  unreachable

777:                                              ; preds = %771, %769
  %778 = phi i64 [ 0, %769 ], [ %766, %771 ]
  %779 = phi ptr [ inttoptr (i64 8 to ptr), %769 ], [ %772, %771 ]
  %780 = icmp ule i64 %766, %778
  call void @llvm.assume(i1 %780)
  store i64 %759, ptr %779, align 8, !noalias !ID
  %781 = getelementptr inbounds nuw i8, ptr %779, i64 8
  store ptr %761, ptr %781, align 8, !noalias !ID
  %782 = getelementptr inbounds nuw i8, ptr %779, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %782, ptr noundef nonnull align 8 dereferenceable(24) %100, i64 24, i1 false), !noalias !ID
  %783 = getelementptr inbounds nuw i8, ptr %779, i64 40
  store i64 %757, ptr %783, align 8, !noalias !ID
  %784 = getelementptr inbounds nuw i8, ptr %779, i64 48
  store i64 %738, ptr %784, align 8, !noalias !ID
  %785 = getelementptr inbounds nuw i8, ptr %779, i64 56
  store ptr %763, ptr %785, align 8, !noalias !ID
  %786 = getelementptr inbounds nuw i8, ptr %779, i64 64
  store i64 %765, ptr %786, align 8, !noalias !ID
  store i64 %778, ptr %101, align 8, !noalias !ID
  %787 = getelementptr inbounds nuw i8, ptr %101, i64 8
  store ptr %779, ptr %787, align 8, !noalias !ID
  %788 = getelementptr inbounds nuw i8, ptr %101, i64 16
  store i64 1, ptr %788, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %99), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %99, ptr noundef nonnull align 8 dereferenceable(64) %143, i64 64, i1 false), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %97), !noalias !ID
  %789 = getelementptr inbounds nuw i8, ptr %99, i64 56
  %790 = load i64, ptr %789, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %98)
  %791 = icmp eq i64 %790, 0
  br i1 %791, label %855, label %792

792:                                              ; preds = %777
  %793 = getelementptr inbounds nuw i8, ptr %99, i64 24
  %794 = getelementptr inbounds nuw i8, ptr %99, i64 48
  %795 = getelementptr inbounds nuw i8, ptr %99, i64 32
  %796 = getelementptr inbounds nuw i8, ptr %97, i64 40
  %797 = getelementptr inbounds nuw i8, ptr %97, i64 48
  %798 = getelementptr inbounds nuw i8, ptr %97, i64 56
  %799 = load i16, ptr %794, align 8, !alias.scope !ID, !noalias !ID
  %800 = load ptr, ptr %793, align 8, !alias.scope !ID, !noalias !ID
  %801 = load ptr, ptr %795, align 8, !alias.scope !ID, !noalias !ID
  br label %802

802:                                              ; preds = %843, %792
  %803 = phi ptr [ %779, %792 ], [ %844, %843 ]
  %804 = phi i64 [ 1, %792 ], [ %846, %843 ]
  %805 = phi ptr [ %800, %792 ], [ %820, %843 ]
  %806 = phi ptr [ %801, %792 ], [ %821, %843 ]
  %807 = phi ptr [ %801, %792 ], [ %822, %843 ]
  %808 = phi ptr [ %800, %792 ], [ %823, %843 ]
  %809 = phi i16 [ %799, %792 ], [ %828, %843 ]
  %810 = phi i64 [ %790, %792 ], [ %831, %843 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %811 = icmp eq i16 %809, 0
  br i1 %811, label %.preheader380, label %.loopexit381

.preheader380:                                    ; preds = %802, %.preheader380
  %812 = phi ptr [ %817, %.preheader380 ], [ %807, %802 ]
  %813 = phi ptr [ %816, %.preheader380 ], [ %808, %802 ]
  %814 = load <16 x i8>, ptr %812, align 16, !noalias !ID
  %815 = icmp sgt <16 x i8> %814, splat (i8 -1)
  %816 = getelementptr inbounds i8, ptr %813, i64 -1152
  %817 = getelementptr inbounds nuw i8, ptr %812, i64 16
  %818 = bitcast <16 x i1> %815 to i16
  %819 = icmp eq i16 %818, 0
  br i1 %819, label %.preheader380, label %.loopexit381

.loopexit381:                                     ; preds = %.preheader380, %802
  %820 = phi ptr [ %805, %802 ], [ %816, %.preheader380 ]
  %821 = phi ptr [ %806, %802 ], [ %817, %.preheader380 ]
  %822 = phi ptr [ %807, %802 ], [ %817, %.preheader380 ]
  %823 = phi ptr [ %808, %802 ], [ %816, %.preheader380 ]
  %824 = phi i16 [ %809, %802 ], [ %818, %.preheader380 ]
  %825 = add i16 %824, -1
  %826 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %824, i1 true)
  %827 = zext nneg i16 %826 to i64
  %828 = and i16 %825, %824
  %829 = sub nsw i64 0, %827
  %830 = getelementptr inbounds [72 x i8], ptr %823, i64 %829
  %831 = add i64 %810, -1
  %832 = getelementptr inbounds i8, ptr %830, i64 -24
  %833 = load i64, ptr %832, align 8, !noalias !ID
  %834 = icmp eq i64 %833, -1
  br i1 %834, label %853, label %835

835:                                              ; preds = %.loopexit381
  %836 = getelementptr inbounds i8, ptr %830, i64 -16
  %837 = getelementptr inbounds i8, ptr %830, i64 -32
  %838 = load i64, ptr %837, align 8, !noalias !ID
  %839 = getelementptr inbounds i8, ptr %830, i64 -72
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %98, ptr noundef nonnull align 8 dereferenceable(16) %836, i64 16, i1 false), !noalias !ID
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %97, ptr noundef nonnull align 8 dereferenceable(40) %839, i64 40, i1 false), !noalias !ID
  store i64 %838, ptr %796, align 8, !noalias !ID
  store i64 %833, ptr %797, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %798, ptr noundef nonnull align 8 dereferenceable(16) %98, i64 16, i1 false), !noalias !ID
  %840 = icmp samesign ult i64 %804, 128102389400760776
  call void @llvm.assume(i1 %840)
  %841 = load i64, ptr %101, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %842 = icmp eq i64 %804, %841
  br i1 %842, label %850, label %843

843:                                              ; preds = %851, %835
  %844 = phi ptr [ %852, %851 ], [ %803, %835 ]
  %845 = getelementptr inbounds nuw [72 x i8], ptr %844, i64 %804
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %845, ptr noundef nonnull align 8 dereferenceable(72) %97, i64 72, i1 false), !noalias !ID
  %846 = add nuw nsw i64 %804, 1
  store i64 %846, ptr %788, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %98)
  call void @llvm.lifetime.start.p0(ptr nonnull %98)
  %847 = icmp eq i64 %831, 0
  br i1 %847, label %853, label %802

848:                                              ; preds = %850
  %849 = landingpad { ptr, i32 }
          cleanup
  store ptr %821, ptr %795, align 8, !noalias !ID
  store ptr %820, ptr %793, align 8, !noalias !ID
  store i16 %828, ptr %794, align 8, !alias.scope !ID, !noalias !ID
  store i64 %831, ptr %789, align 8, !alias.scope !ID, !noalias !ID
; call core::ptr::drop_glue::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>
  call fastcc void @core::ptr::drop_glue::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>(ptr noalias nofree noundef align 8 dereferenceable(72) %97) #ATTR, !noalias !ID
; call core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
  call fastcc void @core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(64) %99), !noalias !ID
; call core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %101) #ATTR, !noalias !ID
  br label %3901

850:                                              ; preds = %835
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.ID)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %101, i64 noundef %804, i64 noundef range(i64 1, 0) %810, i64 noundef 8, i64 noundef 72)
          to label %851 unwind label %848, !noalias !ID

851:                                              ; preds = %850
  %852 = load ptr, ptr %787, align 8, !alias.scope !ID, !noalias !ID
  br label %843

853:                                              ; preds = %843, %.loopexit381
  %854 = phi i64 [ %831, %.loopexit381 ], [ 0, %843 ]
  store ptr %821, ptr %795, align 8, !noalias !ID
  store ptr %820, ptr %793, align 8, !noalias !ID
  store i16 %828, ptr %794, align 8, !alias.scope !ID, !noalias !ID
  store i64 %854, ptr %789, align 8, !alias.scope !ID, !noalias !ID
  br label %855

855:                                              ; preds = %853, %777
  call void @llvm.lifetime.end.p0(ptr nonnull %98)
; call core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>
  call fastcc void @core::ptr::drop_glue::<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>)>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(64) %99), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %97), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %99), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %144, ptr noundef nonnull align 8 dereferenceable(24) %101, i64 24, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %101), !noalias !ID
  %856 = getelementptr inbounds nuw i8, ptr %144, i64 8
  %857 = load ptr, ptr %856, align 8
  %858 = getelementptr inbounds nuw i8, ptr %144, i64 16
  %859 = load i64, ptr %858, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %100)
  call void @llvm.lifetime.end.p0(ptr nonnull %143)
  %860 = icmp samesign ult i64 %859, 2
  br i1 %860, label %877, label %861, !prof !ID

861:                                              ; preds = %855
  %862 = icmp samesign ult i64 %859, 21
  br i1 %862, label %864, label %863, !prof !ID

863:                                              ; preds = %861
; invoke core::slice::sort::unstable::ipnsort::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>
  invoke void @core::slice::sort::unstable::ipnsort::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>(ptr noalias nofree noundef nonnull align 8 %857, i64 noundef range(i64 0, 128102389400760776) %859, ptr noalias nofree nonnull align 8 poison) #ATTR
          to label %877 unwind label %875

864:                                              ; preds = %861
; call core::slice::sort::shared::smallsort::insertion_sort_shift_left::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>
  call fastcc void @core::slice::sort::shared::smallsort::insertion_sort_shift_left::<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>)), <[(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<purrdf_sparql_eval::modifier::ContextualLane<purrdf_core::ir::term::TermId>>))]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::contextual_group_rows<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#3}>::{closure#0}>(ptr noalias nofree noundef nonnull align 8 %857, i64 noundef range(i64 0, 128102389400760776) %859)
  br label %877

865:                                              ; preds = %3419, %._crit_edge2442, %3362, %._crit_edge2448, %1070, %875, %873, %871, %869
  %866 = phi i1 [ %1071, %1070 ], [ true, %._crit_edge2448 ], [ true, %3362 ], [ true, %869 ], [ true, %871 ], [ true, %873 ], [ true, %875 ], [ false, %3419 ], [ false, %._crit_edge2442 ]
  %867 = phi i8 [ %1072, %1070 ], [ %3339, %._crit_edge2448 ], [ %3339, %3362 ], [ 1, %869 ], [ 1, %871 ], [ 1, %873 ], [ 1, %875 ], [ 0, %3419 ], [ 0, %._crit_edge2442 ]
  %868 = phi { ptr, i32 } [ %1073, %1070 ], [ %3354, %._crit_edge2448 ], [ %3354, %3362 ], [ %870, %869 ], [ %872, %871 ], [ %874, %873 ], [ %876, %875 ], [ %3411, %3419 ], [ %3411, %._crit_edge2442 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %144) #ATTR
  br label %3901

869:                                              ; preds = %991, %972, %969
  %870 = landingpad { ptr, i32 }
          cleanup
  br label %865

871:                                              ; preds = %950, %931, %928
  %872 = landingpad { ptr, i32 }
          cleanup
  br label %865

873:                                              ; preds = %1008, %998
  %874 = landingpad { ptr, i32 }
          cleanup
  br label %865

875:                                              ; preds = %.loopexit373, %863
  %876 = landingpad { ptr, i32 }
          cleanup
  br label %865

877:                                              ; preds = %864, %863, %855, %740
  %878 = phi ptr [ %742, %740 ], [ %858, %863 ], [ %858, %855 ], [ %858, %864 ]
  %879 = phi ptr [ %741, %740 ], [ %856, %863 ], [ %856, %855 ], [ %856, %864 ]
  %880 = phi ptr [ inttoptr (i64 8 to ptr), %740 ], [ %857, %863 ], [ %857, %855 ], [ %857, %864 ]
  %881 = phi i64 [ 0, %740 ], [ %859, %863 ], [ %859, %855 ], [ %859, %864 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %142)
  %882 = load i64, ptr %416, align 8, !noundef !ID
  %883 = icmp ult i64 %882, 576460752303423488
  call void @llvm.assume(i1 %883)
  store i64 %882, ptr %142, align 8
  %884 = getelementptr inbounds nuw i8, ptr %7, i64 472
  %885 = load i8, ptr %884, align 8, !range !ID, !noundef !ID
  %886 = icmp eq i8 %885, 2
  br i1 %886, label %887, label %.loopexit373

887:                                              ; preds = %877
  %888 = getelementptr inbounds nuw i8, ptr %7, i64 616
  %889 = load ptr, ptr %888, align 8, !noundef !ID
  %890 = icmp eq ptr %889, null
  br i1 %890, label %905, label %891

891:                                              ; preds = %887
  %892 = getelementptr inbounds nuw i8, ptr %889, i64 24
  %893 = load i64, ptr %892, align 8, !noalias !ID
  %894 = getelementptr inbounds nuw i8, ptr %889, i64 48
  %895 = icmp ult i64 %893, -2
  br i1 %895, label %.loopexit373, label %896

896:                                              ; preds = %891
  %897 = getelementptr inbounds nuw i8, ptr %889, i64 32
  %898 = load i64, ptr %897, align 8, !noalias !ID
  %899 = icmp ult i64 %898, -2
  br i1 %899, label %.loopexit373, label %900

900:                                              ; preds = %896
  %901 = load i64, ptr %894, align 8, !noalias !ID
  %902 = icmp ult i64 %901, -2
  %903 = or i1 %638, %902
  %904 = xor i1 %902, true
  br i1 %903, label %.loopexit373, label %906

905:                                              ; preds = %887
  br i1 %638, label %.loopexit373, label %906

906:                                              ; preds = %905, %900
  %907 = getelementptr inbounds nuw i8, ptr %7, i64 584
  %908 = getelementptr inbounds nuw i8, ptr %7, i64 672
  %909 = getelementptr inbounds nuw i8, ptr %7, i64 680
  %910 = getelementptr inbounds nuw i8, ptr %7, i64 688
  br label %911

911:                                              ; preds = %1022, %906
  %912 = phi ptr [ %5, %906 ], [ %913, %1022 ]
  %913 = getelementptr inbounds nuw i8, ptr %912, i64 120
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %914 = getelementptr inbounds nuw i8, ptr %912, i64 16
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %915 = getelementptr inbounds nuw i8, ptr %912, i64 48
  %916 = load ptr, ptr %915, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %917 = getelementptr inbounds nuw i8, ptr %912, i64 56
  %918 = load i64, ptr %917, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %919 = shl nuw nsw i64 %918, 6
  %920 = getelementptr inbounds nuw i8, ptr %916, i64 %919
  %921 = icmp eq i64 %918, 0
  br i1 %921, label %.loopexit377, label %.preheader375

.preheader375:                                    ; preds = %911, %953
  %922 = phi ptr [ %923, %953 ], [ %916, %911 ]
  %923 = getelementptr inbounds nuw i8, ptr %922, i64 64
  %924 = load ptr, ptr %907, align 8, !noalias !ID, !noundef !ID
  %925 = icmp eq ptr %924, null
  %926 = load ptr, ptr %908, align 16, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
  %927 = load ptr, ptr %909, align 8, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
  br i1 %925, label %931, label %928

928:                                              ; preds = %.preheader375
  call void @llvm.lifetime.start.p0(ptr nonnull %95), !noalias !ID
  store ptr %907, ptr %95, align 8, !noalias !ID
; invoke purrdf_sparql_eval::parallel::reaches_unsafe_builtin
  %929 = invoke fastcc noundef zeroext i1 @purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.ID)(i64 noundef 0, ptr noundef nonnull readonly align 8 dereferenceable(64) %922, ptr nonnull readonly %926, ptr nonnull readonly %927, ptr noundef nonnull %95, ptr nonnull readonly @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0})
          to label %930 unwind label %871

930:                                              ; preds = %928
  call void @llvm.lifetime.end.p0(ptr nonnull %95), !noalias !ID
  br i1 %929, label %.loopexit373, label %934

931:                                              ; preds = %.preheader375
; invoke purrdf_sparql_eval::parallel::reaches_unsafe_builtin
  %932 = invoke fastcc noundef zeroext i1 @purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.ID)(i64 noundef 0, ptr noundef nonnull readonly align 8 dereferenceable(64) %922, ptr nonnull readonly %926, ptr nonnull readonly %927, ptr noundef nonnull inttoptr (i64 1 to ptr), ptr nonnull readonly @purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.ID))
          to label %933 unwind label %871

933:                                              ; preds = %931
  br i1 %932, label %.loopexit373, label %934

934:                                              ; preds = %933, %930
  %935 = load ptr, ptr %888, align 8, !noalias !ID, !noundef !ID
  %936 = icmp eq ptr %935, null
  br i1 %936, label %953, label %937

937:                                              ; preds = %934
  %938 = getelementptr inbounds nuw i8, ptr %935, i64 16
  %939 = getelementptr inbounds nuw i8, ptr %935, i64 336
  %940 = load ptr, ptr %939, align 8, !noalias !ID, !noundef !ID
  %941 = icmp ne ptr %940, null
  %942 = load <4 x i64>, ptr %938, align 8, !alias.scope !ID, !noalias !ID
  %.fr = freeze <4 x i64> %942
  %943 = icmp ne <4 x i64> %.fr, splat (i64 -1)
  %944 = getelementptr inbounds nuw i8, ptr %935, i64 48
  %945 = load i64, ptr %944, align 8, !alias.scope !ID, !noalias !ID
  %946 = icmp ne i64 %945, -1
  %947 = bitcast <4 x i1> %943 to i4
  %948 = icmp ne i4 %947, 0
  %949 = or i1 %941, %948
  %op.rdx2540 = select i1 %949, i1 true, i1 %946
  br i1 %op.rdx2540, label %950, label %953

950:                                              ; preds = %937
; invoke purrdf_sparql_eval::parallel::expression_re_enters_evaluation
  %951 = invoke noundef zeroext i1 @purrdf_sparql_eval::parallel::expression_re_enters_evaluation(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %922)
          to label %952 unwind label %871

952:                                              ; preds = %950
  br i1 %951, label %.loopexit373, label %953

953:                                              ; preds = %952, %937, %934
  %954 = icmp eq ptr %923, %920
  br i1 %954, label %.loopexit377, label %.preheader375

.loopexit377:                                     ; preds = %953, %911
  %955 = getelementptr inbounds nuw i8, ptr %912, i64 96
  %956 = load ptr, ptr %955, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %957 = getelementptr inbounds nuw i8, ptr %912, i64 104
  %958 = load i64, ptr %957, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %959 = mul nuw nsw i64 %958, 72
  %960 = getelementptr inbounds nuw i8, ptr %956, i64 %959
  %961 = icmp eq i64 %958, 0
  br i1 %961, label %.loopexit374, label %.preheader372

.preheader372:                                    ; preds = %.loopexit377, %994
  %962 = phi ptr [ %963, %994 ], [ %956, %.loopexit377 ]
  %963 = getelementptr inbounds nuw i8, ptr %962, i64 72
  %964 = getelementptr inbounds nuw i8, ptr %962, i64 8
  %965 = load ptr, ptr %907, align 8, !noalias !ID, !noundef !ID
  %966 = icmp eq ptr %965, null
  %967 = load ptr, ptr %908, align 16, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
  %968 = load ptr, ptr %909, align 8, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
  br i1 %966, label %972, label %969

969:                                              ; preds = %.preheader372
  call void @llvm.lifetime.start.p0(ptr nonnull %96), !noalias !ID
  store ptr %907, ptr %96, align 8, !noalias !ID
; invoke purrdf_sparql_eval::parallel::reaches_unsafe_builtin
  %970 = invoke fastcc noundef zeroext i1 @purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.ID)(i64 noundef 0, ptr noundef nonnull readonly align 8 dereferenceable(64) %964, ptr nonnull readonly %967, ptr nonnull readonly %968, ptr noundef nonnull %96, ptr nonnull readonly @<purrdf_sparql_eval::eval::EvalCtx>::may_fork_row_loop::{closure#0})
          to label %971 unwind label %869

971:                                              ; preds = %969
  call void @llvm.lifetime.end.p0(ptr nonnull %96), !noalias !ID
  br i1 %970, label %.loopexit373, label %975

972:                                              ; preds = %.preheader372
; invoke purrdf_sparql_eval::parallel::reaches_unsafe_builtin
  %973 = invoke fastcc noundef zeroext i1 @purrdf_sparql_eval::parallel::reaches_unsafe_builtin (.llvm.ID)(i64 noundef 0, ptr noundef nonnull readonly align 8 dereferenceable(64) %964, ptr nonnull readonly %967, ptr nonnull readonly %968, ptr noundef nonnull inttoptr (i64 1 to ptr), ptr nonnull readonly @purrdf_sparql_eval::parallel::is_parallel_safe_pattern::{closure#0} (.llvm.ID))
          to label %974 unwind label %869

974:                                              ; preds = %972
  br i1 %973, label %.loopexit373, label %975

975:                                              ; preds = %974, %971
  %976 = load ptr, ptr %888, align 8, !noalias !ID, !noundef !ID
  %977 = icmp eq ptr %976, null
  br i1 %977, label %994, label %978

978:                                              ; preds = %975
  %979 = getelementptr inbounds nuw i8, ptr %976, i64 16
  %980 = getelementptr inbounds nuw i8, ptr %976, i64 336
  %981 = load ptr, ptr %980, align 8, !noalias !ID, !noundef !ID
  %982 = icmp ne ptr %981, null
  %983 = load <4 x i64>, ptr %979, align 8, !alias.scope !ID, !noalias !ID
  %.fr2541 = freeze <4 x i64> %983
  %984 = icmp ne <4 x i64> %.fr2541, splat (i64 -1)
  %985 = getelementptr inbounds nuw i8, ptr %976, i64 48
  %986 = load i64, ptr %985, align 8, !alias.scope !ID, !noalias !ID
  %987 = icmp ne i64 %986, -1
  %988 = bitcast <4 x i1> %984 to i4
  %989 = icmp ne i4 %988, 0
  %990 = or i1 %982, %989
  %op.rdx2538 = select i1 %990, i1 true, i1 %987
  br i1 %op.rdx2538, label %991, label %994

991:                                              ; preds = %978
; invoke purrdf_sparql_eval::parallel::expression_re_enters_evaluation
  %992 = invoke noundef zeroext i1 @purrdf_sparql_eval::parallel::expression_re_enters_evaluation(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(64) %964)
          to label %993 unwind label %869

993:                                              ; preds = %991
  br i1 %992, label %.loopexit373, label %994

994:                                              ; preds = %993, %978, %975
  %995 = icmp eq ptr %963, %960
  br i1 %995, label %.loopexit374, label %.preheader372

.loopexit374:                                     ; preds = %994, %.loopexit377
  %996 = load i64, ptr %914, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %997 = icmp eq i64 %996, 8
  br i1 %997, label %998, label %1022

998:                                              ; preds = %.loopexit374
  %999 = getelementptr inbounds nuw i8, ptr %912, i64 24
  %1000 = load ptr, ptr %999, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %1001 = getelementptr inbounds nuw i8, ptr %912, i64 32
  %1002 = load i64, ptr %1001, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1003 = getelementptr inbounds nuw i8, ptr %1000, i64 16
  %1004 = load ptr, ptr %910, align 16, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
; invoke <purrdf_sparql_eval::agg_fn::AggregateRegistry>::resolve
  %1005 = invoke noundef align 8 ptr @<purrdf_sparql_eval::agg_fn::AggregateRegistry>::resolve(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(40) %1004, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) %1003, i64 noundef %1002)
          to label %1006 unwind label %873

1006:                                             ; preds = %998
  %1007 = icmp eq ptr %1005, null
  br i1 %1007, label %.loopexit373, label %1008

1008:                                             ; preds = %1006
  %1009 = load ptr, ptr %1005, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %1010 = getelementptr inbounds nuw i8, ptr %1005, i64 8
  %1011 = load ptr, ptr %1010, align 8, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
  %1012 = getelementptr inbounds nuw i8, ptr %1011, i64 16
  %1013 = load i64, ptr %1012, align 8, !range !ID, !invariant.load !ID, !noalias !ID
  %1014 = add nsw i64 %1013, -1
  %1015 = and i64 %1014, -16
  %1016 = getelementptr inbounds nuw i8, ptr %1009, i64 %1015
  %1017 = getelementptr inbounds nuw i8, ptr %1016, i64 16
  %1018 = getelementptr inbounds nuw i8, ptr %1011, i64 32
  %1019 = load ptr, ptr %1018, align 8, !invariant.load !ID, !noalias !ID, !nonnull !ID
  %1020 = invoke noundef zeroext i1 %1019(ptr noundef nonnull %1017) #ATTR
          to label %1021 unwind label %873, !inline_history !ID

1021:                                             ; preds = %1008
  br i1 %1020, label %.loopexit373, label %1022

1022:                                             ; preds = %1021, %.loopexit374
  %1023 = icmp eq ptr %913, %187
  br i1 %1023, label %.loopexit373, label %911

.loopexit373:                                     ; preds = %1022, %1021, %1006, %952, %933, %930, %993, %974, %971, %905, %900, %896, %891, %877
  %1024 = phi i1 [ false, %993 ], [ %904, %900 ], [ false, %952 ], [ false, %877 ], [ true, %905 ], [ false, %896 ], [ false, %891 ], [ false, %971 ], [ false, %974 ], [ false, %930 ], [ false, %933 ], [ false, %1006 ], [ true, %1022 ], [ false, %1021 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %141)
  %1025 = load ptr, ptr %150, align 8, !nonnull !ID, !noundef !ID
  %1026 = getelementptr inbounds nuw i8, ptr %1025, i64 16
; invoke purrdf_sparql_eval::modifier::link_aggregates::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::modifier::link_aggregates::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %141, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %5, i64 noundef %6, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1026, ptr noalias nofree noundef align 16 dereferenceable(1248) %7)
          to label %1027 unwind label %875

1027:                                             ; preds = %.loopexit373
  call void @llvm.lifetime.start.p0(ptr nonnull %140)
  br i1 %1024, label %1298, label %1028

1028:                                             ; preds = %1027
  call void @llvm.lifetime.start.p0(ptr nonnull %116)
  %1029 = icmp ult i64 %881, 128102389400760776
  call void @llvm.assume(i1 %1029)
  %1030 = mul nuw nsw i64 %881, 40
  %1031 = icmp eq i64 %881, 0
  br i1 %1031, label %1032, label %1035

1032:                                             ; preds = %1028
  store i64 0, ptr %116, align 8
  %1033 = getelementptr inbounds nuw i8, ptr %116, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1033, align 8
  %1034 = getelementptr inbounds nuw i8, ptr %116, i64 16
  store i64 0, ptr %1034, align 8
  br label %.loopexit371

1035:                                             ; preds = %1028
  %1036 = call noundef ptr @malloc(i64 noundef range(i64 1, 0) %1030) #ATTR, !noalias !ID
  %1037 = icmp eq ptr %1036, null
  br i1 %1037, label %__rustc::__rust_alloc (.exit292.thread), label %1038

1038:                                             ; preds = %1035
  %1039 = load i64, ptr %196, align 8, !noalias !ID, !noundef !ID
  %1040 = call i64 @llvm.uadd.sat.i64(i64 %1039, i64 1)
  store i64 %1040, ptr %196, align 8, !noalias !ID
  %1041 = load i64, ptr %199, align 8, !noalias !ID, !noundef !ID
  %1042 = call i64 @llvm.uadd.sat.i64(i64 %1041, i64 %1030)
  store i64 %1042, ptr %199, align 8, !noalias !ID
  %1043 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %1044 = call i64 @llvm.sadd.sat.i64(i64 %1043, i64 %1030)
  store i64 %1044, ptr %202, align 8, !noalias !ID
  %1045 = load i64, ptr %205, align 8, !noalias !ID, !noundef !ID
  %1046 = icmp sgt i64 %1044, %1045
  br i1 %1046, label %1047, label %.preheader2950

1047:                                             ; preds = %1038
  store i64 %1044, ptr %205, align 8, !noalias !ID
  br label %.preheader2950

.preheader2950:                                   ; preds = %1047, %1038
  br label %1048

1048:                                             ; preds = %.preheader2950, %1051
  %1049 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %1050 = icmp slt i64 %1049, 0
  br i1 %1050, label %1051, label %__rustc::__rust_alloc (.exit292)

1051:                                             ; preds = %1048
  %1052 = add nsw i64 %1049, 1
  %1053 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %1049, i64 %1052 acq_rel acquire, align 8, !noalias !ID
  %1054 = extractvalue { i64, i1 } %1053, 1
  br i1 %1054, label %1055, label %1048

1055:                                             ; preds = %1051
  %1056 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !ID
  %1057 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %1030 monotonic, align 8, !noalias !ID
  %1058 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1030 monotonic, align 8, !noalias !ID
  %1059 = call i64 @llvm.sadd.sat.i64(i64 %1058, i64 %1030)
  %1060 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !ID
  br label %1061

1061:                                             ; preds = %1064, %1055
  %1062 = phi i64 [ %1060, %1055 ], [ %1067, %1064 ]
  %1063 = icmp sgt i64 %1059, %1062
  br i1 %1063, label %1064, label %1068

1064:                                             ; preds = %1061
  %1065 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %1062, i64 %1059 monotonic monotonic, align 8, !noalias !ID
  %1066 = extractvalue { i64, i1 } %1065, 1
  %1067 = extractvalue { i64, i1 } %1065, 0
  br i1 %1066, label %1068, label %1061

1068:                                             ; preds = %1064, %1061
  %1069 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_alloc (.exit292)

1070:                                             ; preds = %3527, %3303, %1310, %1091, %1074
  %1071 = phi i1 [ true, %1074 ], [ true, %1091 ], [ true, %3527 ], [ false, %3303 ], [ true, %1310 ]
  %1072 = phi i8 [ 1, %1074 ], [ 1, %1091 ], [ 0, %3527 ], [ 0, %3303 ], [ 1, %1310 ]
  %1073 = phi { ptr, i32 } [ %1075, %1074 ], [ %1092, %1091 ], [ %3528, %3527 ], [ %3304, %3303 ], [ %1311, %1310 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %141) #ATTR
          to label %865 unwind label %1290

1074:                                             ; preds = %1298, %3540, %3301, %__rustc::__rust_alloc (.exit292.thread)
  %1075 = landingpad { ptr, i32 }
          cleanup
  br label %1070

__rustc::__rust_alloc (.exit292.thread): ; preds = %1035
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %1030) #ATTR
          to label %1111 unwind label %1074

__rustc::__rust_alloc (.exit292): ; preds = %1048, %1068
  store i64 %881, ptr %116, align 8
  %1076 = getelementptr inbounds nuw i8, ptr %116, i64 8
  store ptr %1036, ptr %1076, align 8
  %1077 = getelementptr inbounds nuw i8, ptr %116, i64 16
  store i64 0, ptr %1077, align 8
  %1078 = mul nuw nsw i64 %881, 72
  %1079 = getelementptr inbounds nuw i8, ptr %880, i64 %1078
  %1080 = getelementptr inbounds nuw i8, ptr %115, i64 8
  %1081 = getelementptr inbounds nuw i8, ptr %115, i64 16
  %1082 = getelementptr inbounds nuw i8, ptr %141, i64 8
  %1083 = getelementptr inbounds nuw i8, ptr %141, i64 16
  %1084 = getelementptr inbounds nuw i8, ptr %114, i64 8
  br label %1085

1085:                                             ; preds = %1164, %__rustc::__rust_alloc (.exit292)
  %1086 = phi ptr [ %1036, %__rustc::__rust_alloc (.exit292) ], [ %1165, %1164 ]
  %1087 = phi i64 [ 0, %__rustc::__rust_alloc (.exit292) ], [ %1169, %1164 ]
  %1088 = phi ptr [ %880, %__rustc::__rust_alloc (.exit292) ], [ %1089, %1164 ]
  %1089 = getelementptr inbounds nuw i8, ptr %1088, i64 72
  call void @llvm.lifetime.start.p0(ptr nonnull %115)
  %1090 = load i64, ptr %142, align 8, !noundef !ID
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem(ptr noalias nofree noundef align 8 captures(none) dereferenceable(40) %115, i64 noundef %1090)
          to label %1095 unwind label %1093

1091:                                             ; preds = %1286, %1282, %1161, %1158, %1093
  %1092 = phi { ptr, i32 } [ %1094, %1093 ], [ %1159, %1161 ], [ %1159, %1158 ], [ %1284, %1282 ], [ %1284, %1286 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %116) #ATTR
  br label %1070

1093:                                             ; preds = %1085
  %1094 = landingpad { ptr, i32 }
          cleanup
  br label %1091

1095:                                             ; preds = %1085
  %1096 = load i64, ptr %115, align 8, !range !ID, !noundef !ID
  %1097 = icmp ugt i64 %1096, 5
  %1098 = load ptr, ptr %1080, align 8, !nonnull !ID
  %1099 = select i1 %1097, ptr %1098, ptr %1080
  %1100 = load i64, ptr %1081, align 8
  %1101 = select i1 %1097, i64 %1100, i64 %1096
  %1102 = add i64 %1101, -1
  %1103 = load i64, ptr %158, align 8, !noundef !ID
  %1104 = icmp ugt i64 %1103, %1102
  br i1 %1104, label %1105, label %1106, !prof !ID

1105:                                             ; preds = %1095
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef 0, i64 noundef %1103, i64 noundef %1102, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.529) #ATTR
          to label %1111 unwind label %1279

1106:                                             ; preds = %1095
  %1107 = load i64, ptr %1088, align 8, !range !ID, !noundef !ID
  %1108 = add i64 %1107, -1
  %1109 = icmp ugt i64 %1108, 4
  %1110 = getelementptr inbounds nuw i8, ptr %1088, i64 8
  br i1 %1109, label %1112, label %1117

1111:                                             ; preds = %3284, %3209, %3194, %1200, %1105, %__rustc::__rust_alloc (.exit292.thread), %420
  unreachable

1112:                                             ; preds = %1106
  %1113 = load ptr, ptr %1110, align 8, !nonnull !ID, !noundef !ID
  %1114 = getelementptr inbounds nuw i8, ptr %1088, i64 16
  %1115 = load i64, ptr %1114, align 8, !noundef !ID
  %1116 = add i64 %1115, -1
  br label %1117

1117:                                             ; preds = %1112, %1106
  %1118 = phi ptr [ %1113, %1112 ], [ %1110, %1106 ]
  %1119 = phi i64 [ %1116, %1112 ], [ %1108, %1106 ]
  %1120 = icmp eq i64 %1103, %1119
  br i1 %1120, label %1123, label %1121, !prof !ID

1121:                                             ; preds = %1117
; invoke core::slice::copy_from_slice_impl::len_mismatch_fail
  invoke void @core::slice::copy_from_slice_impl::len_mismatch_fail(i64 noundef range(i64 0, 1152921504606846976) %1103, i64 noundef range(i64 0, 1152921504606846976) %1119, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.530) #ATTR
          to label %1122 unwind label %1279

1122:                                             ; preds = %1121
  unreachable

1123:                                             ; preds = %1117
  %1124 = shl nuw nsw i64 %1103, 3
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 4 %1099, ptr nonnull readonly align 4 %1118, i64 %1124, i1 false), !alias.scope !ID, !noalias !ID
  %1125 = load ptr, ptr %1082, align 8, !nonnull !ID, !noundef !ID
  %1126 = load i64, ptr %1083, align 8, !noundef !ID
  %1127 = call i64 @llvm.umin.i64(i64 %6, i64 %1126)
  %1128 = icmp eq i64 %1127, 0
  br i1 %1128, label %1150, label %1129

1129:                                             ; preds = %1123
  %1130 = getelementptr inbounds nuw i8, ptr %1088, i64 56
  %1131 = getelementptr inbounds nuw i8, ptr %1088, i64 64
  br label %1132

1132:                                             ; preds = %1195, %1129
  %1133 = phi i64 [ 0, %1129 ], [ %1134, %1195 ]
  %1134 = add nuw nsw i64 %1133, 1
  %1135 = getelementptr inbounds nuw [120 x i8], ptr %5, i64 %1133
  %1136 = getelementptr inbounds nuw [24 x i8], ptr %1125, i64 %1133
  %1137 = getelementptr inbounds nuw i8, ptr %1135, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %114)
  %1138 = getelementptr inbounds nuw i8, ptr %1136, i64 8
  %1139 = load ptr, ptr %1138, align 8, !nonnull !ID, !noundef !ID
  %1140 = getelementptr inbounds nuw i8, ptr %1136, i64 16
  %1141 = load i64, ptr %1140, align 8, !noundef !ID
  %1142 = load ptr, ptr %1130, align 8, !nonnull !ID, !noundef !ID
  %1143 = load i64, ptr %1131, align 8, !noundef !ID
  %1144 = load ptr, ptr %476, align 8, !nonnull !ID, !noundef !ID
  %1145 = load i64, ptr %478, align 8, !noundef !ID
  %1146 = load ptr, ptr %150, align 8, !nonnull !ID, !noundef !ID
  %1147 = getelementptr inbounds nuw i8, ptr %1146, i64 16
; invoke purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
  invoke fastcc void @purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>(ptr noalias nofree noundef align 16 captures(address) dereferenceable(96) %114, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(104) %1137, ptr noalias nofree noundef nonnull align 8 %1139, i64 noundef %1141, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %1142, i64 noundef %1143, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %1144, i64 noundef %1145, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1147, ptr noalias nofree noundef align 16 dereferenceable(1248) %7)
          to label %1171 unwind label %1276

1148:                                             ; preds = %1195
  %1149 = load i64, ptr %115, align 8
  br label %1150

1150:                                             ; preds = %1148, %1123
  %1151 = phi i64 [ %1149, %1148 ], [ %1096, %1123 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %113)
  %1152 = load ptr, ptr %1080, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %113, ptr noundef nonnull align 8 dereferenceable(24) %1081, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1153 = load i64, ptr %116, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1154 = icmp eq i64 %1087, %1153
  br i1 %1154, label %1155, label %1164

1155:                                             ; preds = %1150
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %116)
          to label %1156 unwind label %1158, !noalias !ID

1156:                                             ; preds = %1155
  %1157 = load ptr, ptr %1076, align 8, !alias.scope !ID, !noalias !ID
  br label %1164

1158:                                             ; preds = %1155
  %1159 = landingpad { ptr, i32 }
          cleanup
  %1160 = icmp ugt i64 %1151, 5
  br i1 %1160, label %1161, label %1091

1161:                                             ; preds = %1158
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1152) ]
  %1162 = shl i64 %1151, 3
  %1163 = add i64 %1162, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1152, i64 noundef %1163, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %1091

1164:                                             ; preds = %1156, %1150
  %1165 = phi ptr [ %1157, %1156 ], [ %1086, %1150 ]
  %1166 = getelementptr inbounds nuw [40 x i8], ptr %1165, i64 %1087
  store i64 %1151, ptr %1166, align 8, !noalias !ID
  %1167 = getelementptr inbounds nuw i8, ptr %1166, i64 8
  store ptr %1152, ptr %1167, align 8, !noalias !ID
  %1168 = getelementptr inbounds nuw i8, ptr %1166, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1168, ptr noundef nonnull align 8 dereferenceable(24) %113, i64 24, i1 false), !noalias !ID
  %1169 = add nuw nsw i64 %1087, 1
  store i64 %1169, ptr %1077, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %113)
  call void @llvm.lifetime.end.p0(ptr nonnull %115)
  %1170 = icmp eq ptr %1089, %1079
  br i1 %1170, label %.loopexit371, label %1085

1171:                                             ; preds = %1132
  %1172 = load i64, ptr %114, align 16, !range !ID, !noundef !ID
  %1173 = icmp eq i64 %1172, -1
  %1174 = load <2 x i32>, ptr %1084, align 8
  br i1 %1173, label %1186, label %1175

1175:                                             ; preds = %1171
  %1176 = getelementptr inbounds nuw i8, ptr %114, i64 16
  %1177 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %1177, ptr noundef nonnull align 16 dereferenceable(80) %1176, i64 80, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %114)
  %1178 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %1172, ptr %1178, align 16
  %1179 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store <2 x i32> %1174, ptr %1179, align 8
  store i64 1, ptr %0, align 16
  %1180 = load i64, ptr %115, align 8, !range !ID, !alias.scope !ID, !noundef !ID
  %1181 = icmp ugt i64 %1180, 5
  br i1 %1181, label %1182, label %1201

1182:                                             ; preds = %1175
  %1183 = load ptr, ptr %1080, align 8, !nonnull !ID, !noundef !ID
  %1184 = shl i64 %1180, 3
  %1185 = add i64 %1184, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1183, i64 noundef %1185, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %1201

1186:                                             ; preds = %1171
  call void @llvm.lifetime.end.p0(ptr nonnull %114)
  %1187 = load i64, ptr %158, align 8, !noundef !ID
  %1188 = add i64 %1187, %1133
  %1189 = load i64, ptr %115, align 8, !range !ID, !noundef !ID
  %1190 = icmp ugt i64 %1189, 5
  %1191 = load i64, ptr %1081, align 8
  %1192 = select i1 %1190, i64 %1191, i64 %1189
  %1193 = add i64 %1192, -1
  %1194 = icmp ult i64 %1188, %1193
  br i1 %1194, label %1195, label %1200

1195:                                             ; preds = %1186
  %1196 = load ptr, ptr %1080, align 8, !nonnull !ID
  %1197 = select i1 %1190, ptr %1196, ptr %1080
  %1198 = getelementptr inbounds nuw [8 x i8], ptr %1197, i64 %1188
  store <2 x i32> %1174, ptr %1198, align 4
  %1199 = icmp eq i64 %1134, %1127
  br i1 %1199, label %1148, label %1132

1200:                                             ; preds = %1186
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %1188, i64 noundef %1193, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.531) #ATTR
          to label %1111 unwind label %1279

1201:                                             ; preds = %1182, %1175
  call void @llvm.lifetime.end.p0(ptr nonnull %115)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1202 = icmp eq i64 %1087, 0
  br i1 %1202, label %.loopexit370, label %.preheader369

.preheader369:                                    ; preds = %1201
  %1203 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  br label %1204

1204:                                             ; preds = %.preheader369, %1242
  %1205 = phi i64 [ %1207, %1242 ], [ 0, %.preheader369 ]
  %1206 = getelementptr inbounds nuw [40 x i8], ptr %1086, i64 %1205
  %1207 = add nuw nsw i64 %1205, 1
  %1208 = load i64, ptr %1206, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1209 = icmp ugt i64 %1208, 5
  br i1 %1209, label %1210, label %1242

1210:                                             ; preds = %1204
  %1211 = getelementptr i8, ptr %1206, i64 8
  %1212 = load ptr, ptr %1211, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %1213 = shl i64 %1208, 3
  %1214 = add i64 %1213, -8
  %1215 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %1216 = call i64 @llvm.umin.i64(i64 %1214, i64 9223372036854775807)
  %1217 = call i64 @llvm.ssub.sat.i64(i64 %1215, i64 %1216)
  store i64 %1217, ptr %202, align 8, !noalias !ID
  %1218 = load i64, ptr %1203, align 8, !noalias !ID, !noundef !ID
  %1219 = icmp slt i64 %1217, %1218
  br i1 %1219, label %1220, label %.preheader2890

1220:                                             ; preds = %1210
  store i64 %1217, ptr %1203, align 8, !noalias !ID
  br label %.preheader2890

.preheader2890:                                   ; preds = %1220, %1210
  br label %1221

1221:                                             ; preds = %.preheader2890, %1224
  %1222 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %1223 = icmp slt i64 %1222, 0
  br i1 %1223, label %1224, label %__rustc::__rust_dealloc (.exit293)

1224:                                             ; preds = %1221
  %1225 = add nsw i64 %1222, 1
  %1226 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %1222, i64 %1225 acq_rel acquire, align 8, !noalias !ID
  %1227 = extractvalue { i64, i1 } %1226, 1
  br i1 %1227, label %1228, label %1221

1228:                                             ; preds = %1224
  %1229 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1216 monotonic, align 8, !noalias !ID
  %1230 = call i64 @llvm.ssub.sat.i64(i64 %1229, i64 %1216)
  %1231 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %1232

1232:                                             ; preds = %1235, %1228
  %1233 = phi i64 [ %1231, %1228 ], [ %1238, %1235 ]
  %1234 = icmp slt i64 %1230, %1233
  br i1 %1234, label %1235, label %1239

1235:                                             ; preds = %1232
  %1236 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1233, i64 %1230 monotonic monotonic, align 8, !noalias !ID
  %1237 = extractvalue { i64, i1 } %1236, 1
  %1238 = extractvalue { i64, i1 } %1236, 0
  br i1 %1237, label %1239, label %1232

1239:                                             ; preds = %1235, %1232
  %1240 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit293)

__rustc::__rust_dealloc (.exit293): ; preds = %1221, %1239
  %1241 = icmp ne i64 %1214, 0
  call void @llvm.assume(i1 %1241), !noalias !ID
  call void @free(ptr noundef nonnull %1212) #ATTR, !noalias !ID
  br label %1242

1242:                                             ; preds = %__rustc::__rust_dealloc (.exit293), %1204
  %1243 = icmp eq i64 %1207, %1087
  br i1 %1243, label %.loopexit370, label %1204

.loopexit370:                                     ; preds = %1242, %1201
  %1244 = load i64, ptr %116, align 8, !alias.scope !ID
  %1245 = icmp eq i64 %1244, 0
  br i1 %1245, label %1275, label %1246

1246:                                             ; preds = %.loopexit370
  %1247 = mul nuw i64 %1244, 40
  %1248 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %1249 = call i64 @llvm.umin.i64(i64 %1247, i64 9223372036854775807)
  %1250 = call i64 @llvm.ssub.sat.i64(i64 %1248, i64 %1249)
  store i64 %1250, ptr %202, align 8, !noalias !ID
  %1251 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %1252 = load i64, ptr %1251, align 8, !noalias !ID, !noundef !ID
  %1253 = icmp slt i64 %1250, %1252
  br i1 %1253, label %1254, label %.preheader2889

1254:                                             ; preds = %1246
  store i64 %1250, ptr %1251, align 8, !noalias !ID
  br label %.preheader2889

.preheader2889:                                   ; preds = %1254, %1246
  br label %1255

1255:                                             ; preds = %.preheader2889, %1258
  %1256 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %1257 = icmp slt i64 %1256, 0
  br i1 %1257, label %1258, label %__rustc::__rust_dealloc (.exit294)

1258:                                             ; preds = %1255
  %1259 = add nsw i64 %1256, 1
  %1260 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %1256, i64 %1259 acq_rel acquire, align 8, !noalias !ID
  %1261 = extractvalue { i64, i1 } %1260, 1
  br i1 %1261, label %1262, label %1255

1262:                                             ; preds = %1258
  %1263 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1249 monotonic, align 8, !noalias !ID
  %1264 = call i64 @llvm.ssub.sat.i64(i64 %1263, i64 %1249)
  %1265 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %1266

1266:                                             ; preds = %1269, %1262
  %1267 = phi i64 [ %1265, %1262 ], [ %1272, %1269 ]
  %1268 = icmp slt i64 %1264, %1267
  br i1 %1268, label %1269, label %1273

1269:                                             ; preds = %1266
  %1270 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1267, i64 %1264 monotonic monotonic, align 8, !noalias !ID
  %1271 = extractvalue { i64, i1 } %1270, 1
  %1272 = extractvalue { i64, i1 } %1270, 0
  br i1 %1271, label %1273, label %1266

1273:                                             ; preds = %1269, %1266
  %1274 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit294)

__rustc::__rust_dealloc (.exit294): ; preds = %1255, %1273
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1086) ], !noalias !ID
  call void @free(ptr noundef nonnull %1086) #ATTR, !noalias !ID
  br label %1275

1275:                                             ; preds = %__rustc::__rust_dealloc (.exit294), %.loopexit370
  call void @llvm.lifetime.end.p0(ptr nonnull %116)
  br label %3338

1276:                                             ; preds = %1132
  %1277 = landingpad { ptr, i32 }
          cleanup
  %1278 = load i64, ptr %115, align 8, !range !ID, !alias.scope !ID
  br label %1282

1279:                                             ; preds = %1200, %1121, %1105
  %1280 = phi i64 [ %1096, %1121 ], [ %1189, %1200 ], [ %1096, %1105 ]
  %1281 = landingpad { ptr, i32 }
          cleanup
  br label %1282

1282:                                             ; preds = %1279, %1276
  %1283 = phi i64 [ %1278, %1276 ], [ %1280, %1279 ]
  %1284 = phi { ptr, i32 } [ %1277, %1276 ], [ %1281, %1279 ]
  %1285 = icmp ugt i64 %1283, 5
  br i1 %1285, label %1286, label %1091

1286:                                             ; preds = %1282
  %1287 = load ptr, ptr %1080, align 8, !nonnull !ID, !noundef !ID
  %1288 = shl i64 %1283, 3
  %1289 = add i64 %1288, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1287, i64 noundef %1289, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %1091

1290:                                             ; preds = %4025, %4020, %4016, %3538, %3536, %3517, %1332, %1310, %1070, %409
  %1291 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR
  unreachable

.loopexit371:                                     ; preds = %1164, %1032
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %140, ptr noundef nonnull align 8 dereferenceable(24) %116, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %116)
  br label %1292

1292:                                             ; preds = %3302, %.loopexit371
  %1293 = getelementptr inbounds nuw i8, ptr %7, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1294 = load ptr, ptr %1293, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %1295 = getelementptr inbounds nuw i8, ptr %1294, i64 40
  %1296 = load atomic i32, ptr %1295 acquire, align 4, !noalias !ID
  %1297 = icmp eq i32 %1296, 0
  br i1 %1297, label %3305, label %3312

1298:                                             ; preds = %1027
  call void @llvm.lifetime.start.p0(ptr nonnull %139)
  %1299 = getelementptr inbounds nuw i8, ptr %7, i64 888
  %1300 = getelementptr inbounds nuw i8, ptr %7, i64 904
  %1301 = load i64, ptr %1300, align 8, !noundef !ID
  %1302 = getelementptr inbounds nuw i8, ptr %7, i64 1040
  %1303 = load i64, ptr %1302, align 16, !noundef !ID
  %1304 = icmp ult i64 %1301, 115292150460684698
  call void @llvm.assume(i1 %1304)
  %1305 = add i64 %1303, %1301
  store i64 %1305, ptr %139, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %138)
  %1306 = getelementptr inbounds nuw i8, ptr %7, i64 616
  %.val = load ptr, ptr %1306, align 8, !noundef !ID
; invoke <purrdf_sparql_eval::row_checkpoint::ItemLedger>::for_items::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::ItemLedger>::for_items::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(168) %138, ptr %.val)
          to label %1307 unwind label %1074

1307:                                             ; preds = %1298
  call void @llvm.lifetime.start.p0(ptr nonnull %137)
  %1308 = icmp ult i64 %881, 128102389400760776
  call void @llvm.assume(i1 %1308)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot
  %1309 = invoke fastcc noundef ptr @<purrdf_sparql_eval::eval::EvalCtx>::loop_snapshot(ptr noundef nonnull align 16 %7, i64 noundef %881)
          to label %1314 unwind label %1312

1310:                                             ; preds = %1332, %1329, %1325, %1312
  %1311 = phi { ptr, i32 } [ %1313, %1312 ], [ %1326, %1332 ], [ %1326, %1325 ], [ %1326, %1329 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(168) %138)
          to label %1070 unwind label %1290

1312:                                             ; preds = %3535, %3182, %1307
  %1313 = landingpad { ptr, i32 }
          cleanup
  br label %1310

1314:                                             ; preds = %1307
  store ptr %1309, ptr %137, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %136)
  call void @llvm.lifetime.start.p0(ptr nonnull %135)
  %1315 = load ptr, ptr %1306, align 8, !noundef !ID
  %1316 = icmp eq ptr %1315, null
  br i1 %1316, label %1335, label %1317

1317:                                             ; preds = %1314
  %1318 = getelementptr inbounds nuw i8, ptr %1315, i64 16
  %1319 = load i64, ptr %1318, align 8
  %1320 = icmp ugt i64 %1319, -3
  br i1 %1320, label %1321, label %1335

1321:                                             ; preds = %1317
  %1322 = getelementptr inbounds nuw i8, ptr %1315, i64 40
  %1323 = load i64, ptr %1322, align 8
  %1324 = icmp ult i64 %1323, -2
  br label %1335

1325:                                             ; preds = %3538, %3536, %3166, %1772, %1532, %1528, %1438, %1333
  %1326 = phi { ptr, i32 } [ %3539, %3538 ], [ %1773, %1772 ], [ %3537, %3536 ], [ %3167, %3166 ], [ %1439, %1438 ], [ %1334, %1333 ], [ %1529, %1532 ], [ %1529, %1528 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1327 = load ptr, ptr %137, align 8, !alias.scope !ID, !noundef !ID
  %1328 = icmp eq ptr %1327, null
  br i1 %1328, label %1310, label %1329

1329:                                             ; preds = %1325
  %1330 = atomicrmw sub ptr %1327, i64 1 release, align 8, !noalias !ID
  %1331 = icmp eq i64 %1330, 1
  br i1 %1331, label %1332, label %1310

1332:                                             ; preds = %1329
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %137) #ATTR
          to label %1310 unwind label %1290, !inline_history !ID

1333:                                             ; preds = %1461, %1364, %1463, %1449, %1442, %1357
  %1334 = landingpad { ptr, i32 }
          cleanup
  br label %1325

1335:                                             ; preds = %1321, %1317, %1314
  %1336 = phi i1 [ false, %1314 ], [ true, %1317 ], [ %1324, %1321 ]
  %1337 = getelementptr inbounds nuw i8, ptr %7, i64 1234
  %1338 = load i8, ptr %1337, align 2, !range !ID, !noundef !ID
  %1339 = trunc nuw i8 %1338 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %134)
  store ptr %141, ptr %134, align 8
  %1340 = getelementptr inbounds nuw i8, ptr %134, i64 8
  store ptr %7, ptr %1340, align 8
  %1341 = getelementptr inbounds nuw i8, ptr %134, i64 16
  store ptr %137, ptr %1341, align 8
  %1342 = getelementptr inbounds nuw i8, ptr %134, i64 24
  store ptr %138, ptr %1342, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %133)
  store ptr %142, ptr %133, align 8
  %1343 = getelementptr inbounds nuw i8, ptr %133, i64 8
  store ptr %158, ptr %1343, align 8
  %1344 = getelementptr inbounds nuw i8, ptr %133, i64 16
  store ptr %5, ptr %1344, align 8
  %1345 = getelementptr inbounds nuw i8, ptr %133, i64 24
  store i64 %6, ptr %1345, align 8
  %1346 = getelementptr inbounds nuw i8, ptr %133, i64 32
  store ptr %153, ptr %1346, align 8
  %1347 = getelementptr inbounds nuw i8, ptr %133, i64 40
  store ptr %150, ptr %1347, align 8
  %1348 = getelementptr inbounds nuw i8, ptr %133, i64 48
  store ptr %9, ptr %1348, align 8
  %1349 = getelementptr inbounds nuw i8, ptr %133, i64 56
  store ptr %139, ptr %1349, align 8
  br i1 %1336, label %1446, label %1350

1350:                                             ; preds = %1335
  call void @llvm.lifetime.start.p0(ptr nonnull %35)
  call void @llvm.lifetime.start.p0(ptr nonnull %36)
  store ptr %880, ptr %36, align 8, !noalias !ID
  %1351 = getelementptr inbounds nuw i8, ptr %36, i64 8
  store i64 %881, ptr %1351, align 8, !noalias !ID
  store ptr %138, ptr %35, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !ID
  store ptr %134, ptr %34, align 8, !noalias !ID
  %1352 = getelementptr inbounds nuw i8, ptr %34, i64 8
  store ptr %36, ptr %1352, align 8, !noalias !ID
  %1353 = getelementptr inbounds nuw i8, ptr %34, i64 16
  store ptr %133, ptr %1353, align 8, !noalias !ID
  %1354 = getelementptr inbounds nuw i8, ptr %34, i64 24
  store ptr %35, ptr %1354, align 8, !noalias !ID
  %1355 = icmp samesign ult i64 %881, 1025
  %1356 = or i1 %1355, %1339
  br i1 %1356, label %1357, label %1358

1357:                                             ; preds = %1350
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(224) %135, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) %34) #ATTR
          to label %1445 unwind label %1333, !inline_history !ID

1358:                                             ; preds = %1350
  %1359 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %1360 = load ptr, ptr %1359, align 8, !noundef !ID
  %1361 = icmp eq ptr %1360, null
  br i1 %1361, label %1364, label %1362

1362:                                             ; preds = %1358
  %1363 = getelementptr inbounds nuw i8, ptr %1360, i64 272
  br label %1366

1364:                                             ; preds = %1358
; invoke rayon_core::registry::global_registry
  %1365 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %._crit_edge unwind label %1333

._crit_edge:                                      ; preds = %1364
  %.pre = load ptr, ptr %36, align 8, !noalias !ID
  %.pre1210 = load i64, ptr %1351, align 8, !noalias !ID
  br label %1366

1366:                                             ; preds = %._crit_edge, %1362
  %1367 = phi i64 [ %881, %1362 ], [ %.pre1210, %._crit_edge ]
  %1368 = phi ptr [ %880, %1362 ], [ %.pre, %._crit_edge ]
  %1369 = phi ptr [ %1363, %1362 ], [ %1365, %._crit_edge ]
  %1370 = load ptr, ptr %1369, align 8, !nonnull !ID, !noundef !ID
  %1371 = getelementptr inbounds nuw i8, ptr %1370, i64 520
  %1372 = load i64, ptr %1371, align 8, !noundef !ID
  %1373 = icmp ult i64 %1372, 192153584101141163
  call void @llvm.assume(i1 %1373)
  %1374 = call i64 @llvm.umax.i64(i64 %1372, i64 1)
  %1375 = shl nuw nsw i64 %1374, 2
  %1376 = udiv i64 %881, %1375
  %1377 = call noundef range(i64 16, 0) i64 @llvm.umax.i64(i64 %1376, i64 16)
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !ID
  store i64 0, ptr %32, align 8, !alias.scope !ID, !noalias !ID
  %1378 = getelementptr inbounds nuw i8, ptr %32, i64 8
  store ptr inttoptr (i64 16 to ptr), ptr %1378, align 8, !alias.scope !ID, !noalias !ID
  %1379 = getelementptr inbounds nuw i8, ptr %32, i64 16
  store i64 0, ptr %1379, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1380 = udiv i64 %1367, %1377
  %1381 = urem i64 %1367, %1377
  %1382 = icmp ne i64 %1381, 0
  %1383 = zext i1 %1382 to i64
  %1384 = add nuw nsw i64 %1380, %1383
  call void @llvm.experimental.noalias.scope.decl(metadata !ID), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !ID
  store i64 %1384, ptr %31, align 8, !noalias !ID
  %1385 = icmp eq i64 %1384, 0
  br i1 %1385, label %1390, label %1386, !prof !ID

1386:                                             ; preds = %1366
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.ID)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %32, i64 noundef 0, i64 noundef %1384, i64 noundef 16, i64 noundef 224)
          to label %1387 unwind label %1436, !noalias !ID, !inline_history !ID

1387:                                             ; preds = %1386
  %1388 = load i64, ptr %1379, align 8, !alias.scope !ID, !noalias !ID
  %1389 = load i64, ptr %32, align 8, !range !ID, !alias.scope !ID, !noalias !ID
  br label %1390

1390:                                             ; preds = %1387, %1366
  %1391 = phi i64 [ %1389, %1387 ], [ 0, %1366 ]
  %1392 = phi i64 [ %1388, %1387 ], [ 0, %1366 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !ID
  %1393 = icmp ult i64 %1392, 41175768021673107
  call void @llvm.assume(i1 %1393)
  %1394 = sub nsw i64 %1391, %1392
  %1395 = icmp ult i64 %1394, %1384
  br i1 %1395, label %1396, label %1398, !prof !ID

1396:                                             ; preds = %1390
; invoke core::panicking::panic
  invoke void @core::panicking::panic(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.HASH.1066, i64 noundef 47, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.1068) #ATTR
          to label %1397 unwind label %1436, !noalias !ID, !inline_history !ID

1397:                                             ; preds = %1396
  unreachable

1398:                                             ; preds = %1390
  %1399 = load ptr, ptr %1378, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !ID
  store ptr %1368, ptr %27, align 8, !noalias !ID
  %1400 = getelementptr inbounds nuw i8, ptr %27, i64 8
  store i64 %1367, ptr %1400, align 8, !noalias !ID
  %1401 = getelementptr inbounds nuw i8, ptr %27, i64 16
  store i64 %1377, ptr %1401, align 8, !noalias !ID
  %1402 = getelementptr inbounds nuw i8, ptr %27, i64 24
  store ptr %134, ptr %1402, align 8, !noalias !ID
  %1403 = getelementptr inbounds nuw i8, ptr %27, i64 32
  store ptr %133, ptr %1403, align 8, !noalias !ID
  %1404 = getelementptr inbounds nuw i8, ptr %27, i64 40
  store ptr %35, ptr %1404, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !ID
  %1405 = getelementptr inbounds nuw i8, ptr %26, i64 16
  store i64 %1377, ptr %1405, align 8, !noalias !ID
  store ptr %1368, ptr %26, align 8, !noalias !ID
  %1406 = getelementptr inbounds nuw i8, ptr %26, i64 8
  store i64 %1367, ptr %1406, align 8, !noalias !ID
  %1407 = load ptr, ptr %1359, align 8, !noalias !ID, !noundef !ID
  %1408 = icmp eq ptr %1407, null
  br i1 %1408, label %1411, label %1409

1409:                                             ; preds = %1398
  %1410 = getelementptr inbounds nuw i8, ptr %1407, i64 272
  br label %1413

1411:                                             ; preds = %1398
; invoke rayon_core::registry::global_registry
  %1412 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %1413 unwind label %1436

1413:                                             ; preds = %1409, %1411
  %1414 = phi ptr [ %1410, %1409 ], [ %1412, %1411 ]
  %1415 = load ptr, ptr %1414, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %1416 = getelementptr inbounds nuw i8, ptr %1415, i64 520
  %1417 = load i64, ptr %1416, align 8, !noalias !ID, !noundef !ID
  %1418 = icmp ult i64 %1417, 192153584101141163
  call void @llvm.assume(i1 %1418), !noalias !ID
  %1419 = getelementptr inbounds nuw [224 x i8], ptr %1399, i64 %1392
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !ID
  store ptr %1402, ptr %25, align 8, !noalias !ID
  %1420 = getelementptr inbounds nuw i8, ptr %25, i64 8
  store ptr %1419, ptr %1420, align 8, !noalias !ID
  %1421 = getelementptr inbounds nuw i8, ptr %25, i64 16
  store i64 %1384, ptr %1421, align 8, !noalias !ID
; invoke rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
  invoke fastcc void @rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::slice::chunks::ChunksProducer<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>, purrdf_sparql_eval::parallel::par_chunk_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>(ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %30, i64 noundef %1384, i1 noundef zeroext false, i64 noundef %1417, i64 noundef 1, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(24) %26, ptr noalias nofree noundef readonly align 8 captures(address) dereferenceable(24) %25)
          to label %1422 unwind label %1436, !noalias !ID, !inline_history !ID

1422:                                             ; preds = %1413
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %27), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !ID
  %1423 = getelementptr inbounds nuw i8, ptr %30, i64 16
  %1424 = load i64, ptr %1423, align 8, !noalias !ID, !noundef !ID
  store i64 %1424, ptr %29, align 8, !noalias !ID
  %1425 = icmp eq i64 %1424, %1384
  br i1 %1425, label %1442, label %1426, !prof !ID

1426:                                             ; preds = %1422
  call void @llvm.lifetime.start.p0(ptr nonnull %28), !noalias !ID
  store ptr %31, ptr %28, align 8, !noalias !ID
  %1427 = getelementptr inbounds nuw i8, ptr %28, i64 8
  store ptr @<usize as core::fmt::Display>::fmt, ptr %1427, align 8, !noalias !ID
  %1428 = getelementptr inbounds nuw i8, ptr %28, i64 16
  store ptr %29, ptr %1428, align 8, !noalias !ID
  %1429 = getelementptr inbounds nuw i8, ptr %28, i64 24
  store ptr @<usize as core::fmt::Display>::fmt, ptr %1429, align 8, !noalias !ID
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.HASH.619, ptr noundef nonnull %28, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.621) #ATTR
          to label %1433 unwind label %1430, !noalias !ID, !inline_history !ID

1430:                                             ; preds = %1426
  %1431 = landingpad { ptr, i32 }
          cleanup
  %1432 = load ptr, ptr %30, align 8, !noalias !ID, !noundef !ID
; invoke core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<rayon::iter::collect::consumer::CollectResult<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>(ptr %1432, i64 %1424) #ATTR
          to label %1438 unwind label %1434, !noalias !ID, !inline_history !ID

1433:                                             ; preds = %1426
  unreachable

1434:                                             ; preds = %1430
  %1435 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID, !inline_history !ID
  unreachable

1436:                                             ; preds = %1411, %1413, %1396, %1386
  %1437 = landingpad { ptr, i32 }
          cleanup
  br label %1438

1438:                                             ; preds = %1436, %1430
  %1439 = phi { ptr, i32 } [ %1437, %1436 ], [ %1431, %1430 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %32) #ATTR
          to label %1325 unwind label %1440, !noalias !ID, !inline_history !ID

1440:                                             ; preds = %1438
  %1441 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID, !inline_history !ID
  unreachable

1442:                                             ; preds = %1422
  %1443 = add nuw nsw i64 %1392, %1384
  store i64 %1443, ptr %1379, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %33, ptr noundef nonnull align 8 dereferenceable(24) %32, i64 24, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !ID
; invoke purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
  invoke fastcc void @purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(224) %135, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %33)
          to label %1444 unwind label %1333, !inline_history !ID

1444:                                             ; preds = %1442
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !ID
  br label %1445

1445:                                             ; preds = %1444, %1357
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  call void @llvm.lifetime.end.p0(ptr nonnull %36)
  br label %1739

1446:                                             ; preds = %1335
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  store ptr %138, ptr %24, align 8, !noalias !ID
  %1447 = icmp samesign ult i64 %881, 1025
  %1448 = or i1 %1447, %1339
  br i1 %1448, label %1449, label %1455

1449:                                             ; preds = %1446
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %23, ptr noundef nonnull readonly align 8 dereferenceable(32) %134, i64 32, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(64) %22, ptr noundef nonnull readonly align 8 dereferenceable(64) %133, i64 64, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !ID
  store ptr %880, ptr %14, align 8, !noalias !ID
  %1450 = getelementptr inbounds nuw i8, ptr %14, i64 8
  store i64 %881, ptr %1450, align 8, !noalias !ID
  store ptr %138, ptr %13, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !ID
  store ptr %23, ptr %12, align 8, !noalias !ID
  %1451 = getelementptr inbounds nuw i8, ptr %12, i64 8
  store ptr %14, ptr %1451, align 8, !noalias !ID
  %1452 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store ptr %22, ptr %1452, align 8, !noalias !ID
  %1453 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr %13, ptr %1453, align 8, !noalias !ID
; invoke purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}
  invoke fastcc void @purrdf_sparql_eval::parallel::par_chunk_try_map_init::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#0}(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(224) %135, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(32) %12) #ATTR
          to label %1454 unwind label %1333, !inline_history !ID

1454:                                             ; preds = %1449
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !ID
  br label %1738

1455:                                             ; preds = %1446
  %1456 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @rayon_core::registry::WORKER_THREAD_STATE::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL)
  %1457 = load ptr, ptr %1456, align 8, !noundef !ID
  %1458 = icmp eq ptr %1457, null
  br i1 %1458, label %1461, label %1459

1459:                                             ; preds = %1455
  %1460 = getelementptr inbounds nuw i8, ptr %1457, i64 272
  br label %1463

1461:                                             ; preds = %1455
; invoke rayon_core::registry::global_registry
  %1462 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %1463 unwind label %1333

1463:                                             ; preds = %1459, %1461
  %1464 = phi ptr [ %1460, %1459 ], [ %1462, %1461 ]
  %1465 = load ptr, ptr %1464, align 8, !nonnull !ID, !noundef !ID
  %1466 = getelementptr inbounds nuw i8, ptr %1465, i64 520
  %1467 = load i64, ptr %1466, align 8, !noundef !ID
  %1468 = icmp ult i64 %1467, 192153584101141163
  call void @llvm.assume(i1 %1468)
  %1469 = call i64 @llvm.umax.i64(i64 %1467, i64 1)
  %1470 = shl nuw i64 %1469, 6
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !ID
  %1471 = udiv i64 %881, %1470
  %1472 = call i64 @llvm.umax.i64(i64 %1471, i64 64)
  store ptr %880, ptr %20, align 8, !noalias !ID
  %1473 = getelementptr inbounds nuw i8, ptr %20, i64 8
  store i64 %881, ptr %1473, align 8, !noalias !ID
  %1474 = getelementptr inbounds nuw i8, ptr %20, i64 16
  store i64 %1472, ptr %1474, align 8, !noalias !ID
; invoke <alloc::vec::Vec<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)]> as alloc::vec::spec_from_iter::SpecFromIter<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)], core::slice::iter::Chunks<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>>::from_iter
  invoke fastcc void @<alloc::vec::Vec<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)]> as alloc::vec::spec_from_iter::SpecFromIter<&[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)], core::slice::iter::Chunks<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>)>>>::from_iter(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %21, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %20)
          to label %1475 unwind label %1333, !inline_history !ID

1475:                                             ; preds = %1463
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !ID
  %1476 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %1477 = getelementptr inbounds nuw i8, ptr %21, i64 16
  %1478 = load i64, ptr %1477, align 8, !noalias !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1479 = mul i64 %1478, 240
  %1480 = icmp ugt i64 %1478, 38430716820228232
  br i1 %1480, label %1486, label %1481, !prof !ID

1481:                                             ; preds = %1475
  %1482 = icmp eq i64 %1479, 0
  br i1 %1482, label %1489, label %1483

1483:                                             ; preds = %1481
; call __rustc::__rust_alloc
  %1484 = call noundef align 16 ptr @__rustc::__rust_alloc(i64 noundef %1479, i64 noundef range(i64 1, 17) 16) #ATTR, !noalias !ID, !inline_history !ID
  %1485 = icmp eq ptr %1484, null
  br i1 %1485, label %1486, label %1489

1486:                                             ; preds = %1483, %1475
  %1487 = phi i64 [ 16, %1483 ], [ 0, %1475 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %1487, i64 %1479) #ATTR
          to label %1488 unwind label %1535, !noalias !ID, !inline_history !ID

1488:                                             ; preds = %1486
  unreachable

1489:                                             ; preds = %1483, %1481
  %1490 = phi i64 [ 0, %1481 ], [ %1478, %1483 ]
  %1491 = phi ptr [ inttoptr (i64 16 to ptr), %1481 ], [ %1484, %1483 ]
  %1492 = icmp samesign ule i64 %1478, %1490
  call void @llvm.assume(i1 %1492)
  %1493 = icmp eq i64 %1478, 0
  br i1 %1493, label %1543, label %.preheader368.preheader

.preheader368.preheader:                          ; preds = %1489
  %xtraiter = and i64 %1478, 7
  %1494 = icmp ult i64 %1478, 8
  br i1 %1494, label %.preheader368.epil.preheader, label %.preheader368.preheader.new

.preheader368.preheader.new:                      ; preds = %.preheader368.preheader
  %unroll_iter = and i64 %1478, 72057594037927928
  br label %.preheader368

.preheader368:                                    ; preds = %.preheader368, %.preheader368.preheader.new
  %1495 = phi i64 [ 0, %.preheader368.preheader.new ], [ %1527, %.preheader368 ]
  %niter = phi i64 [ 0, %.preheader368.preheader.new ], [ %niter.next.7, %.preheader368 ]
  %1496 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  store i32 0, ptr %1496, align 16, !noalias !ID
  %1497 = getelementptr inbounds nuw i8, ptr %1496, i64 4
  store i8 0, ptr %1497, align 4, !noalias !ID
  %1498 = getelementptr inbounds nuw i8, ptr %1496, i64 16
  store i64 2, ptr %1498, align 16, !noalias !ID
  %1499 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  %1500 = getelementptr inbounds nuw i8, ptr %1499, i64 240
  store i32 0, ptr %1500, align 16, !noalias !ID
  %1501 = getelementptr inbounds nuw i8, ptr %1499, i64 244
  store i8 0, ptr %1501, align 4, !noalias !ID
  %1502 = getelementptr inbounds nuw i8, ptr %1499, i64 256
  store i64 2, ptr %1502, align 16, !noalias !ID
  %1503 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  %1504 = getelementptr inbounds nuw i8, ptr %1503, i64 480
  store i32 0, ptr %1504, align 16, !noalias !ID
  %1505 = getelementptr inbounds nuw i8, ptr %1503, i64 484
  store i8 0, ptr %1505, align 4, !noalias !ID
  %1506 = getelementptr inbounds nuw i8, ptr %1503, i64 496
  store i64 2, ptr %1506, align 16, !noalias !ID
  %1507 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  %1508 = getelementptr inbounds nuw i8, ptr %1507, i64 720
  store i32 0, ptr %1508, align 16, !noalias !ID
  %1509 = getelementptr inbounds nuw i8, ptr %1507, i64 724
  store i8 0, ptr %1509, align 4, !noalias !ID
  %1510 = getelementptr inbounds nuw i8, ptr %1507, i64 736
  store i64 2, ptr %1510, align 16, !noalias !ID
  %1511 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  %1512 = getelementptr inbounds nuw i8, ptr %1511, i64 960
  store i32 0, ptr %1512, align 16, !noalias !ID
  %1513 = getelementptr inbounds nuw i8, ptr %1511, i64 964
  store i8 0, ptr %1513, align 4, !noalias !ID
  %1514 = getelementptr inbounds nuw i8, ptr %1511, i64 976
  store i64 2, ptr %1514, align 16, !noalias !ID
  %1515 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  %1516 = getelementptr inbounds nuw i8, ptr %1515, i64 1200
  store i32 0, ptr %1516, align 16, !noalias !ID
  %1517 = getelementptr inbounds nuw i8, ptr %1515, i64 1204
  store i8 0, ptr %1517, align 4, !noalias !ID
  %1518 = getelementptr inbounds nuw i8, ptr %1515, i64 1216
  store i64 2, ptr %1518, align 16, !noalias !ID
  %1519 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  %1520 = getelementptr inbounds nuw i8, ptr %1519, i64 1440
  store i32 0, ptr %1520, align 16, !noalias !ID
  %1521 = getelementptr inbounds nuw i8, ptr %1519, i64 1444
  store i8 0, ptr %1521, align 4, !noalias !ID
  %1522 = getelementptr inbounds nuw i8, ptr %1519, i64 1456
  store i64 2, ptr %1522, align 16, !noalias !ID
  %1523 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1495
  %1524 = getelementptr inbounds nuw i8, ptr %1523, i64 1680
  store i32 0, ptr %1524, align 16, !noalias !ID
  %1525 = getelementptr inbounds nuw i8, ptr %1523, i64 1684
  store i8 0, ptr %1525, align 4, !noalias !ID
  %1526 = getelementptr inbounds nuw i8, ptr %1523, i64 1696
  store i64 2, ptr %1526, align 16, !noalias !ID
  %1527 = add nuw i64 %1495, 8
  %niter.next.7 = add i64 %niter, 8
  %niter.ncmp.7 = icmp eq i64 %niter.next.7, %unroll_iter
  br i1 %niter.ncmp.7, label %.unr-lcssa, label %.preheader368

1528:                                             ; preds = %1734, %1597, %1566, %1535
  %1529 = phi { ptr, i32 } [ %1735, %1734 ], [ %1567, %1566 ], [ %1536, %1535 ], [ %1705, %1597 ]
  %1530 = load i64, ptr %21, align 8, !noalias !ID
  %1531 = icmp eq i64 %1530, 0
  br i1 %1531, label %1325, label %1532

1532:                                             ; preds = %1528
  %1533 = load ptr, ptr %1476, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %1534 = shl nuw i64 %1530, 4
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1533, i64 noundef %1534, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID, !inline_history !ID
  br label %1325

1535:                                             ; preds = %1486
  %1536 = landingpad { ptr, i32 }
          cleanup
  br label %1528

.unr-lcssa:                                       ; preds = %.preheader368
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.epilog-lcssa, label %.preheader368.epil.preheader

.preheader368.epil.preheader:                     ; preds = %.unr-lcssa, %.preheader368.preheader
  %.epil.init = phi i64 [ 0, %.preheader368.preheader ], [ %1527, %.unr-lcssa ]
  %lcmp.mod2997 = icmp ne i64 %xtraiter, 0
  call void @llvm.assume(i1 %lcmp.mod2997)
  br label %.preheader368.epil

.preheader368.epil:                               ; preds = %.preheader368.epil, %.preheader368.epil.preheader
  %1537 = phi i64 [ %1541, %.preheader368.epil ], [ %.epil.init, %.preheader368.epil.preheader ]
  %epil.iter = phi i64 [ %epil.iter.next, %.preheader368.epil ], [ 0, %.preheader368.epil.preheader ]
  %1538 = getelementptr inbounds nuw [240 x i8], ptr %1491, i64 %1537
  store i32 0, ptr %1538, align 16, !noalias !ID
  %1539 = getelementptr inbounds nuw i8, ptr %1538, i64 4
  store i8 0, ptr %1539, align 4, !noalias !ID
  %1540 = getelementptr inbounds nuw i8, ptr %1538, i64 16
  store i64 2, ptr %1540, align 16, !noalias !ID
  %1541 = add nuw i64 %1537, 1
  %epil.iter.next = add i64 %epil.iter, 1
  %epil.iter.cmp.not = icmp eq i64 %epil.iter.next, %xtraiter
  br i1 %epil.iter.cmp.not, label %.epilog-lcssa, label %.preheader368.epil, !llvm.loop !ID

.epilog-lcssa:                                    ; preds = %.preheader368.epil, %.unr-lcssa
  %1542 = load i64, ptr %1477, align 8, !noalias !ID
  br label %1543

1543:                                             ; preds = %.epilog-lcssa, %1489
  %1544 = phi i64 [ 0, %1489 ], [ %1542, %.epilog-lcssa ]
  store i64 %1490, ptr %19, align 8, !alias.scope !ID, !noalias !ID
  %1545 = getelementptr inbounds nuw i8, ptr %19, i64 8
  store ptr %1491, ptr %1545, align 8, !alias.scope !ID, !noalias !ID
  %1546 = getelementptr inbounds nuw i8, ptr %19, i64 16
  store i64 %1478, ptr %1546, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !ID
  store i64 0, ptr %18, align 8, !noalias !ID
  %1547 = icmp ult i64 %1544, 576460752303423488
  call void @llvm.assume(i1 %1547)
  %1548 = call i64 @llvm.umin.i64(i64 %1469, i64 %1544)
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !ID
  store ptr %18, ptr %17, align 8, !noalias !ID
  %1549 = getelementptr inbounds nuw i8, ptr %17, i64 8
  store ptr %21, ptr %1549, align 8, !noalias !ID
  %1550 = getelementptr inbounds nuw i8, ptr %17, i64 16
  store ptr %134, ptr %1550, align 8, !noalias !ID
  %1551 = getelementptr inbounds nuw i8, ptr %17, i64 24
  store ptr %133, ptr %1551, align 8, !noalias !ID
  %1552 = getelementptr inbounds nuw i8, ptr %17, i64 32
  store ptr %24, ptr %1552, align 8, !noalias !ID
  %1553 = getelementptr inbounds nuw i8, ptr %17, i64 40
  store ptr %19, ptr %1553, align 8, !noalias !ID
  %1554 = load ptr, ptr %1456, align 8, !noalias !ID, !noundef !ID
  %1555 = icmp eq ptr %1554, null
  br i1 %1555, label %1558, label %1556

1556:                                             ; preds = %1543
  %1557 = getelementptr inbounds nuw i8, ptr %1554, i64 272
  br label %1560

1558:                                             ; preds = %1543
; invoke rayon_core::registry::global_registry
  %1559 = invoke noundef nonnull align 8 ptr @rayon_core::registry::global_registry()
          to label %1560 unwind label %1734

1560:                                             ; preds = %1556, %1558
  %1561 = phi ptr [ %1557, %1556 ], [ %1559, %1558 ]
  %1562 = load ptr, ptr %1561, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %1563 = getelementptr inbounds nuw i8, ptr %1562, i64 520
  %1564 = load i64, ptr %1563, align 8, !noalias !ID, !noundef !ID
  %1565 = icmp ult i64 %1564, 192153584101141163
  call void @llvm.assume(i1 %1565), !noalias !ID
; invoke rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::range::IterProducer<usize>, rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>
  invoke fastcc void @rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::range::IterProducer<usize>, rayon::iter::for_each::ForEachConsumer<purrdf_sparql_eval::parallel::par_blocks_try_map_init<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, alloc::vec::Vec<usize>), (purrdf_sparql_eval::eval::EvalCtx, alloc::vec::Vec<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger), purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#6}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#7}, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, ()>::{closure#8}>::{closure#1}>>(i64 noundef %1548, i1 noundef zeroext false, i64 noundef %1564, i64 noundef 1, i64 noundef 0, i64 noundef range(i64 0, 576460752303423488) %1548, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(48) %17)
          to label %1568 unwind label %1734, !noalias !ID, !inline_history !ID

1566:                                             ; preds = %1726, %1722
  %1567 = landingpad { ptr, i32 }
          cleanup
  br label %1528

1568:                                             ; preds = %1560
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !ID
  %1569 = load ptr, ptr %1545, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %1570 = load i64, ptr %19, align 8, !range !ID, !noalias !ID, !noundef !ID
  %1571 = load i64, ptr %1546, align 8, !noalias !ID, !noundef !ID
  %1572 = icmp ult i64 %1571, 38430716820228233
  call void @llvm.assume(i1 %1572)
  %1573 = mul nuw i64 %1571, 240
  %1574 = getelementptr inbounds nuw i8, ptr %1569, i64 %1573
  %1575 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %1576 = getelementptr inbounds nuw i8, ptr %15, i64 16
  %1577 = getelementptr inbounds nuw i8, ptr %15, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1578 = mul i64 %1570, 240
  %1579 = udiv i64 %1578, 224
  %1580 = icmp eq i64 %1571, 0
  br i1 %1580, label %.loopexit367, label %.preheader366.preheader

.preheader366.preheader:                          ; preds = %1568
  %1581 = add i64 %1573, -240
  %1582 = udiv i64 %1581, 240
  %1583 = add nuw nsw i64 %1582, 1
  %xtraiter2998 = and i64 %1583, 7
  %lcmp.mod2999.not = icmp eq i64 %xtraiter2998, 0
  br i1 %lcmp.mod2999.not, label %.preheader366.prol.loopexit, label %.preheader366.prol

.preheader366.prol:                               ; preds = %.preheader366.preheader, %1594
  %1584 = phi ptr [ %1589, %1594 ], [ %1569, %.preheader366.preheader ]
  %1585 = phi ptr [ %1595, %1594 ], [ %1569, %.preheader366.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %1594 ], [ 0, %.preheader366.preheader ]
  %1586 = getelementptr inbounds nuw i8, ptr %1584, i64 16
  %1587 = load i64, ptr %1586, align 16, !noalias !ID
  %1588 = getelementptr inbounds nuw i8, ptr %1584, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1588, i64 216, i1 false), !noalias !ID
  %1589 = getelementptr inbounds nuw i8, ptr %1584, i64 240
  %1590 = icmp eq i64 %1587, 2
  br i1 %1590, label %1594, label %1591

1591:                                             ; preds = %.preheader366.prol
  store i64 %1587, ptr %1585, align 16, !noalias !ID
  %1592 = getelementptr inbounds nuw i8, ptr %1585, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1592, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1593 = getelementptr inbounds nuw i8, ptr %1585, i64 224
  br label %1594

1594:                                             ; preds = %1591, %.preheader366.prol
  %1595 = phi ptr [ %1593, %1591 ], [ %1585, %.preheader366.prol ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %prol.iter.next = add i64 %prol.iter, 1
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter2998
  br i1 %prol.iter.cmp.not, label %.preheader366.prol.loopexit, label %.preheader366.prol, !llvm.loop !ID

.preheader366.prol.loopexit:                      ; preds = %1594, %.preheader366.preheader
  %.lcssa2888.unr = phi ptr [ poison, %.preheader366.preheader ], [ %1595, %1594 ]
  %.unr3000 = phi ptr [ %1569, %.preheader366.preheader ], [ %1589, %1594 ]
  %.unr3001 = phi ptr [ %1569, %.preheader366.preheader ], [ %1595, %1594 ]
  %1596 = icmp ult i64 %1581, 1680
  br i1 %1596, label %.loopexit367, label %.preheader366

1597:                                             ; preds = %.loopexit363
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %15)
          to label %1528 unwind label %1720, !noalias !ID, !inline_history !ID

.preheader366:                                    ; preds = %.preheader366.prol.loopexit, %1664
  %1598 = phi ptr [ %1659, %1664 ], [ %.unr3000, %.preheader366.prol.loopexit ]
  %1599 = phi ptr [ %1665, %1664 ], [ %.unr3001, %.preheader366.prol.loopexit ]
  %1600 = getelementptr inbounds nuw i8, ptr %1598, i64 16
  %1601 = load i64, ptr %1600, align 16, !noalias !ID
  %1602 = getelementptr inbounds nuw i8, ptr %1598, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1602, i64 216, i1 false), !noalias !ID
  %1603 = icmp eq i64 %1601, 2
  br i1 %1603, label %.preheader366.1, label %1604

1604:                                             ; preds = %.preheader366
  store i64 %1601, ptr %1599, align 16, !noalias !ID
  %1605 = getelementptr inbounds nuw i8, ptr %1599, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1605, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1606 = getelementptr inbounds nuw i8, ptr %1599, i64 224
  br label %.preheader366.1

.preheader366.1:                                  ; preds = %1604, %.preheader366
  %1607 = phi ptr [ %1606, %1604 ], [ %1599, %.preheader366 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1608 = getelementptr inbounds nuw i8, ptr %1598, i64 256
  %1609 = load i64, ptr %1608, align 16, !noalias !ID
  %1610 = getelementptr inbounds nuw i8, ptr %1598, i64 264
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1610, i64 216, i1 false), !noalias !ID
  %1611 = icmp eq i64 %1609, 2
  br i1 %1611, label %.preheader366.2, label %1612

1612:                                             ; preds = %.preheader366.1
  store i64 %1609, ptr %1607, align 16, !noalias !ID
  %1613 = getelementptr inbounds nuw i8, ptr %1607, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1613, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1614 = getelementptr inbounds nuw i8, ptr %1607, i64 224
  br label %.preheader366.2

.preheader366.2:                                  ; preds = %1612, %.preheader366.1
  %1615 = phi ptr [ %1614, %1612 ], [ %1607, %.preheader366.1 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1616 = getelementptr inbounds nuw i8, ptr %1598, i64 496
  %1617 = load i64, ptr %1616, align 16, !noalias !ID
  %1618 = getelementptr inbounds nuw i8, ptr %1598, i64 504
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1618, i64 216, i1 false), !noalias !ID
  %1619 = icmp eq i64 %1617, 2
  br i1 %1619, label %.preheader366.3, label %1620

1620:                                             ; preds = %.preheader366.2
  store i64 %1617, ptr %1615, align 16, !noalias !ID
  %1621 = getelementptr inbounds nuw i8, ptr %1615, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1621, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1622 = getelementptr inbounds nuw i8, ptr %1615, i64 224
  br label %.preheader366.3

.preheader366.3:                                  ; preds = %1620, %.preheader366.2
  %1623 = phi ptr [ %1622, %1620 ], [ %1615, %.preheader366.2 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1624 = getelementptr inbounds nuw i8, ptr %1598, i64 736
  %1625 = load i64, ptr %1624, align 16, !noalias !ID
  %1626 = getelementptr inbounds nuw i8, ptr %1598, i64 744
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1626, i64 216, i1 false), !noalias !ID
  %1627 = icmp eq i64 %1625, 2
  br i1 %1627, label %.preheader366.4, label %1628

1628:                                             ; preds = %.preheader366.3
  store i64 %1625, ptr %1623, align 16, !noalias !ID
  %1629 = getelementptr inbounds nuw i8, ptr %1623, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1629, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1630 = getelementptr inbounds nuw i8, ptr %1623, i64 224
  br label %.preheader366.4

.preheader366.4:                                  ; preds = %1628, %.preheader366.3
  %1631 = phi ptr [ %1630, %1628 ], [ %1623, %.preheader366.3 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1632 = getelementptr inbounds nuw i8, ptr %1598, i64 976
  %1633 = load i64, ptr %1632, align 16, !noalias !ID
  %1634 = getelementptr inbounds nuw i8, ptr %1598, i64 984
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1634, i64 216, i1 false), !noalias !ID
  %1635 = icmp eq i64 %1633, 2
  br i1 %1635, label %.preheader366.5, label %1636

1636:                                             ; preds = %.preheader366.4
  store i64 %1633, ptr %1631, align 16, !noalias !ID
  %1637 = getelementptr inbounds nuw i8, ptr %1631, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1637, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1638 = getelementptr inbounds nuw i8, ptr %1631, i64 224
  br label %.preheader366.5

.preheader366.5:                                  ; preds = %1636, %.preheader366.4
  %1639 = phi ptr [ %1638, %1636 ], [ %1631, %.preheader366.4 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1640 = getelementptr inbounds nuw i8, ptr %1598, i64 1216
  %1641 = load i64, ptr %1640, align 16, !noalias !ID
  %1642 = getelementptr inbounds nuw i8, ptr %1598, i64 1224
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1642, i64 216, i1 false), !noalias !ID
  %1643 = icmp eq i64 %1641, 2
  br i1 %1643, label %.preheader366.6, label %1644

1644:                                             ; preds = %.preheader366.5
  store i64 %1641, ptr %1639, align 16, !noalias !ID
  %1645 = getelementptr inbounds nuw i8, ptr %1639, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1645, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1646 = getelementptr inbounds nuw i8, ptr %1639, i64 224
  br label %.preheader366.6

.preheader366.6:                                  ; preds = %1644, %.preheader366.5
  %1647 = phi ptr [ %1646, %1644 ], [ %1639, %.preheader366.5 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1648 = getelementptr inbounds nuw i8, ptr %1598, i64 1456
  %1649 = load i64, ptr %1648, align 16, !noalias !ID
  %1650 = getelementptr inbounds nuw i8, ptr %1598, i64 1464
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1650, i64 216, i1 false), !noalias !ID
  %1651 = icmp eq i64 %1649, 2
  br i1 %1651, label %.preheader366.7, label %1652

1652:                                             ; preds = %.preheader366.6
  store i64 %1649, ptr %1647, align 16, !noalias !ID
  %1653 = getelementptr inbounds nuw i8, ptr %1647, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1653, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1654 = getelementptr inbounds nuw i8, ptr %1647, i64 224
  br label %.preheader366.7

.preheader366.7:                                  ; preds = %1652, %.preheader366.6
  %1655 = phi ptr [ %1654, %1652 ], [ %1647, %.preheader366.6 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1656 = getelementptr inbounds nuw i8, ptr %1598, i64 1696
  %1657 = load i64, ptr %1656, align 16, !noalias !ID
  %1658 = getelementptr inbounds nuw i8, ptr %1598, i64 1704
  call void @llvm.lifetime.start.p0(ptr nonnull %10)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %10, ptr noundef nonnull align 8 dereferenceable(216) %1658, i64 216, i1 false), !noalias !ID
  %1659 = getelementptr inbounds nuw i8, ptr %1598, i64 1920
  %1660 = icmp eq i64 %1657, 2
  br i1 %1660, label %1664, label %1661

1661:                                             ; preds = %.preheader366.7
  store i64 %1657, ptr %1655, align 16, !noalias !ID
  %1662 = getelementptr inbounds nuw i8, ptr %1655, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(216) %1662, ptr noundef nonnull align 8 dereferenceable(216) %10, i64 216, i1 false), !noalias !ID
  %1663 = getelementptr inbounds nuw i8, ptr %1655, i64 224
  br label %1664

1664:                                             ; preds = %1661, %.preheader366.7
  %1665 = phi ptr [ %1663, %1661 ], [ %1655, %.preheader366.7 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %10)
  %1666 = icmp eq ptr %1659, %1574
  br i1 %1666, label %.loopexit367, label %.preheader366

.loopexit367:                                     ; preds = %.preheader366.prol.loopexit, %1664, %1568
  %1667 = phi ptr [ %1569, %1568 ], [ %1574, %1664 ], [ %1574, %.preheader366.prol.loopexit ]
  %1668 = phi ptr [ %1569, %1568 ], [ %.lcssa2888.unr, %.preheader366.prol.loopexit ], [ %1665, %1664 ]
  %1669 = ptrtoint ptr %1668 to i64
  %1670 = ptrtoint ptr %1569 to i64
  %1671 = sub nuw i64 %1669, %1670
  %1672 = udiv exact i64 %1671, 224
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !ID
  store ptr %1569, ptr %11, align 8, !noalias !ID
  %1673 = getelementptr inbounds nuw i8, ptr %11, i64 8
  store i64 %1672, ptr %1673, align 8, !noalias !ID
  %1674 = getelementptr inbounds nuw i8, ptr %11, i64 16
  store i64 %1570, ptr %1674, align 8, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1675 = ptrtoint ptr %1574 to i64
  %1676 = ptrtoint ptr %1667 to i64
  %1677 = sub nuw i64 %1675, %1676
  %1678 = udiv exact i64 %1677, 240
  store i64 0, ptr %1576, align 8, !alias.scope !ID, !noalias !ID
  store ptr inttoptr (i64 16 to ptr), ptr %15, align 8, !alias.scope !ID, !noalias !ID
  store ptr inttoptr (i64 16 to ptr), ptr %1575, align 8, !alias.scope !ID, !noalias !ID
  store ptr inttoptr (i64 16 to ptr), ptr %1577, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1679 = icmp eq ptr %1574, %1667
  br i1 %1679, label %.loopexit365, label %.preheader364

.preheader364:                                    ; preds = %.loopexit367, %1687
  %1680 = phi i64 [ %1682, %1687 ], [ 0, %.loopexit367 ]
  %1681 = getelementptr inbounds nuw [240 x i8], ptr %1667, i64 %1680
  %1682 = add nuw nsw i64 %1680, 1
  %1683 = getelementptr inbounds nuw i8, ptr %1681, i64 16
  %1684 = load i64, ptr %1683, align 16, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1685 = icmp eq i64 %1684, 2
  br i1 %1685, label %1687, label %1686

1686:                                             ; preds = %.preheader364
; invoke core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef nonnull readonly align 16 dereferenceable(224) %1683)
          to label %1687 unwind label %1689, !noalias !ID, !inline_history !ID

1687:                                             ; preds = %1686, %.preheader364
  %1688 = icmp eq i64 %1682, %1678
  br i1 %1688, label %.loopexit365, label %.preheader364

1689:                                             ; preds = %1686
  %1690 = landingpad { ptr, i32 }
          cleanup
  %1691 = icmp eq i64 %1682, %1678
  br i1 %1691, label %.loopexit363, label %.preheader362

.preheader362:                                    ; preds = %1689, %1699
  %1692 = phi i64 [ %1694, %1699 ], [ %1682, %1689 ]
  %1693 = getelementptr inbounds nuw [240 x i8], ptr %1667, i64 %1692
  %1694 = add i64 %1692, 1
  %1695 = getelementptr inbounds nuw i8, ptr %1693, i64 16
  %1696 = load i64, ptr %1695, align 16, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1697 = icmp eq i64 %1696, 2
  br i1 %1697, label %1699, label %1698

1698:                                             ; preds = %.preheader362
; invoke core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef nonnull readonly align 16 dereferenceable(224) %1695)
          to label %1699 unwind label %1701, !noalias !ID, !inline_history !ID

1699:                                             ; preds = %1698, %.preheader362
  %1700 = icmp eq i64 %1694, %1678
  br i1 %1700, label %.loopexit363, label %.preheader362

1701:                                             ; preds = %1698
  %1702 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID, !inline_history !ID
  unreachable

1703:                                             ; preds = %1718
  %1704 = landingpad { ptr, i32 }
          cleanup
  br label %.loopexit363

.loopexit363:                                     ; preds = %1699, %1703, %1689
  %1705 = phi { ptr, i32 } [ %1704, %1703 ], [ %1690, %1689 ], [ %1690, %1699 ]
; invoke core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>, core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %11) #ATTR
          to label %1597 unwind label %1720, !noalias !ID, !inline_history !ID

.loopexit365:                                     ; preds = %1687, %.loopexit367
  %1706 = icmp ne i64 %1570, 0
  %1707 = mul nuw i64 %1579, 224
  %1708 = icmp ne i64 %1578, %1707
  %1709 = select i1 %1706, i1 %1708, i1 false
  br i1 %1709, label %1710, label %1722

1710:                                             ; preds = %.loopexit365
  %1711 = icmp ult i64 %1578, 224
  br i1 %1711, label %1712, label %1715

1712:                                             ; preds = %1710
  %1713 = icmp eq i64 %1578, 0
  br i1 %1713, label %1722, label %1714

1714:                                             ; preds = %1712
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1569, i64 noundef %1578, i64 noundef 16) #ATTR, !noalias !ID, !inline_history !ID
  br label %1722

1715:                                             ; preds = %1710
  %1716 = icmp ule i64 %1707, %1578
  call void @llvm.assume(i1 %1716)
; call <purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc
  %_0.i = call noalias noundef align 16 ptr @<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @qualification_454_native_cost::GLOBAL (.llvm.ID), ptr noundef nonnull %1569, i64 noundef 16, i64 noundef %1578, i64 noundef %1707) #ATTR, !noalias !ID
  %1717 = icmp eq ptr %_0.i, null
  br i1 %1717, label %1718, label %1722, !prof !ID

1718:                                             ; preds = %1715
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 16, i64 noundef %1707) #ATTR
          to label %1719 unwind label %1703, !noalias !ID, !inline_history !ID

1719:                                             ; preds = %1718
  unreachable

1720:                                             ; preds = %.loopexit363, %1597
  %1721 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID, !inline_history !ID
  unreachable

1722:                                             ; preds = %1715, %1714, %1712, %.loopexit365
  %1723 = phi ptr [ inttoptr (i64 16 to ptr), %1712 ], [ %_0.i, %1715 ], [ %1569, %.loopexit365 ], [ inttoptr (i64 16 to ptr), %1714 ]
  store i64 %1579, ptr %16, align 8, !alias.scope !ID, !noalias !ID
  %1724 = getelementptr inbounds nuw i8, ptr %16, i64 8
  store ptr %1723, ptr %1724, align 8, !alias.scope !ID, !noalias !ID
  %1725 = getelementptr inbounds nuw i8, ptr %16, i64 16
  store i64 %1672, ptr %1725, align 8, !alias.scope !ID, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %15)
          to label %1726 unwind label %1566, !noalias !ID, !inline_history !ID

1726:                                             ; preds = %1722
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !ID
; invoke purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>
  invoke fastcc void @purrdf_sparql_eval::parallel::concatenate_chunks::<purrdf_sparql_eval::parallel::MintedRow, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(224) %135, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %16)
          to label %1727 unwind label %1566, !inline_history !ID

1727:                                             ; preds = %1726
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !ID
  %1728 = load i64, ptr %21, align 8, !noalias !ID
  %1729 = icmp eq i64 %1728, 0
  br i1 %1729, label %1733, label %1730

1730:                                             ; preds = %1727
  %1731 = load ptr, ptr %1476, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %1732 = shl nuw i64 %1728, 4
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1731, i64 noundef %1732, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID, !inline_history !ID
  br label %1733

1733:                                             ; preds = %1730, %1727
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !ID
  br label %1738

1734:                                             ; preds = %1558, %1560
  %1735 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<std::sync::poison::mutex::Mutex<core::option::Option<core::result::Result<(alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>, (purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger)), purrdf_sparql_eval::error::EvalError>>>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %19) #ATTR
          to label %1528 unwind label %1736, !noalias !ID, !inline_history !ID

1736:                                             ; preds = %1734
  %1737 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID, !inline_history !ID
  unreachable

1738:                                             ; preds = %1733, %1454
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  br label %1739

1739:                                             ; preds = %1738, %1445
  call void @llvm.lifetime.end.p0(ptr nonnull %133)
  call void @llvm.lifetime.end.p0(ptr nonnull %134)
  %1740 = load i64, ptr %135, align 16, !range !ID, !noundef !ID
  %1741 = icmp eq i64 %1740, -1
  br i1 %1741, label %1742, label %1748

1742:                                             ; preds = %1739
  %1743 = getelementptr inbounds nuw i8, ptr %135, i64 16
  %1744 = getelementptr inbounds nuw i8, ptr %135, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %136, ptr noundef nonnull align 16 dereferenceable(64) %1744, i64 64, i1 false)
  %1745 = getelementptr inbounds nuw i8, ptr %0, i64 48
  %1746 = getelementptr inbounds nuw i8, ptr %0, i64 16
  %1747 = load <4 x i64>, ptr %1743, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %135)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %1745, ptr noundef nonnull align 16 dereferenceable(64) %136, i64 64, i1 false)
  store <4 x i64> %1747, ptr %1746, align 16
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %136)
  br label %3529

1748:                                             ; preds = %1739
  %1749 = getelementptr inbounds nuw i8, ptr %135, i64 8
  %1750 = getelementptr inbounds nuw i8, ptr %135, i64 16
  %1751 = getelementptr inbounds nuw i8, ptr %135, i64 24
  %1752 = load i64, ptr %1751, align 8
  %1753 = getelementptr inbounds nuw i8, ptr %135, i64 32
  %1754 = load i64, ptr %1753, align 16
  %1755 = getelementptr inbounds nuw i8, ptr %135, i64 40
  %1756 = load i64, ptr %1755, align 8
  %1757 = getelementptr inbounds nuw i8, ptr %135, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(176) %136, ptr noundef nonnull align 16 dereferenceable(176) %1757, i64 176, i1 false)
  %1758 = getelementptr inbounds nuw i8, ptr %130, i64 24
  %1759 = getelementptr inbounds nuw i8, ptr %124, i64 8
  %1760 = load i64, ptr %1750, align 16
  %1761 = load <2 x i64>, ptr %1749, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %135)
  call void @llvm.lifetime.start.p0(ptr nonnull %130)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(176) %1758, ptr noundef nonnull align 16 dereferenceable(176) %136, i64 176, i1 false)
  store i64 %1740, ptr %124, align 8
  store <2 x i64> %1761, ptr %1759, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %136)
  call void @llvm.lifetime.start.p0(ptr nonnull %131)
  %1762 = add i64 %1752, -3
  %1763 = icmp ult i64 %1762, -2
  %1764 = select i1 %1763, i64 %1756, i64 %1752
  %1765 = add i64 %1764, -1
  %1766 = select i1 %1763, i64 %1752, i64 1
  %1767 = select i1 %1763, i64 1, i64 %1756
  store i64 %1766, ptr %130, align 8
  %1768 = getelementptr inbounds nuw i8, ptr %130, i64 8
  store i64 %1754, ptr %1768, align 8
  %1769 = getelementptr inbounds nuw i8, ptr %130, i64 16
  store i64 %1767, ptr %1769, align 8
  %1770 = getelementptr inbounds nuw i8, ptr %130, i64 200
  store i64 0, ptr %1770, align 8
  %1771 = getelementptr inbounds nuw i8, ptr %130, i64 208
  store i64 %1765, ptr %1771, align 8
; invoke <purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
  invoke fastcc void @<purrdf_core::small::IntoIter<[(purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger); 1]> as core::iter::traits::iterator::Iterator>::unzip::<purrdf_sparql_eval::witness::RelationWitness, purrdf_sparql_eval::row_checkpoint::ItemLedger, purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(208) %131, ptr noalias nofree noundef align 8 captures(address) dereferenceable(216) %130)
          to label %1776 unwind label %3538

1772:                                             ; preds = %3132
  %1773 = landingpad { ptr, i32 }
          cleanup
  br label %1325

1774:                                             ; preds = %3139
  %1775 = landingpad { ptr, i32 }
          cleanup
  br label %3536

1776:                                             ; preds = %1748
  call void @llvm.lifetime.end.p0(ptr nonnull %130)
  call void @llvm.lifetime.start.p0(ptr nonnull %132)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %132, ptr noundef nonnull align 8 dereferenceable(32) %131, i64 32, i1 false)
  %1777 = getelementptr inbounds nuw i8, ptr %131, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(176) %123, ptr noundef nonnull align 8 dereferenceable(176) %1777, i64 176, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %131)
  %1778 = load ptr, ptr %1306, align 8, !noundef !ID
  %1779 = icmp eq ptr %1778, null
  br i1 %1779, label %3092, label %1780

1780:                                             ; preds = %1776
  call void @llvm.lifetime.start.p0(ptr nonnull %122)
  call void @llvm.lifetime.start.p0(ptr nonnull %121)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %121, ptr noundef nonnull align 8 dereferenceable(24) %124, i64 24, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %120)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(176) %120, ptr noundef nonnull align 8 dereferenceable(176) %123, i64 176, i1 false)
  %1781 = getelementptr inbounds nuw i8, ptr %138, i64 160
  %1782 = load i8, ptr %1781, align 8, !range !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1783 = trunc nuw i8 %1782 to i1
  br i1 %1783, label %1905, label %1784

1784:                                             ; preds = %1780
  call void @llvm.lifetime.start.p0(ptr nonnull %94)
  call void @llvm.lifetime.start.p0(ptr nonnull %93), !noalias !ID
  store i64 0, ptr %93, align 8, !noalias !ID
  %1785 = getelementptr inbounds nuw i8, ptr %93, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1785, align 8, !noalias !ID
  %1786 = getelementptr inbounds nuw i8, ptr %93, i64 16
  store i64 0, ptr %1786, align 8, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %86), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %85), !noalias !ID
  %1787 = getelementptr inbounds nuw i8, ptr %121, i64 16
  %1788 = load i64, ptr %1787, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1789 = icmp ult i64 %1788, 230584300921369396
  call void @llvm.assume(i1 %1789)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %85, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(24) %93, i64 noundef %1788)
          to label %1790 unwind label %1903, !noalias !ID

1790:                                             ; preds = %1784
  %1791 = load i64, ptr %85, align 16, !range !ID, !noalias !ID, !noundef !ID
  %1792 = icmp eq i64 %1791, -1
  %1793 = getelementptr inbounds nuw i8, ptr %85, i64 8
  %1794 = load i64, ptr %1793, align 8, !noalias !ID
  %1795 = getelementptr inbounds nuw i8, ptr %85, i64 16
  %1796 = load ptr, ptr %1795, align 16, !noalias !ID
  %1797 = getelementptr inbounds nuw i8, ptr %85, i64 24
  %1798 = load i64, ptr %1797, align 8, !noalias !ID
  br i1 %1792, label %1805, label %1799

1799:                                             ; preds = %1790
  %1800 = getelementptr inbounds nuw i8, ptr %85, i64 32
  %1801 = load i64, ptr %1800, align 16, !noalias !ID
  %1802 = getelementptr inbounds nuw i8, ptr %85, i64 40
  %1803 = load i64, ptr %1802, align 8, !noalias !ID
  %1804 = getelementptr inbounds nuw i8, ptr %85, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %94, ptr noundef nonnull align 16 dereferenceable(48) %1804, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %85), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %86), !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %124)
          to label %1933 unwind label %1930

1805:                                             ; preds = %1790
  call void @llvm.lifetime.end.p0(ptr nonnull %85), !noalias !ID
  store i64 %1794, ptr %86, align 8, !noalias !ID
  %1806 = getelementptr inbounds nuw i8, ptr %86, i64 8
  store ptr %1796, ptr %1806, align 8, !noalias !ID
  %1807 = getelementptr inbounds nuw i8, ptr %86, i64 16
  store i64 %1798, ptr %1807, align 8, !noalias !ID
  %1808 = getelementptr inbounds nuw i8, ptr %121, i64 8
  %1809 = load ptr, ptr %1808, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %1810 = load i64, ptr %121, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1811 = mul nuw nsw i64 %1788, 40
  %1812 = getelementptr inbounds nuw i8, ptr %1809, i64 %1811
  call void @llvm.lifetime.start.p0(ptr nonnull %84), !noalias !ID
  store ptr %1809, ptr %84, align 8, !noalias !ID
  %1813 = getelementptr inbounds nuw i8, ptr %84, i64 8
  %1814 = getelementptr inbounds nuw i8, ptr %84, i64 16
  store i64 %1810, ptr %1814, align 8, !noalias !ID
  %1815 = getelementptr inbounds nuw i8, ptr %84, i64 24
  store ptr %1812, ptr %1815, align 8, !noalias !ID
  %1816 = icmp eq i64 %1788, 0
  br i1 %1816, label %.loopexit361, label %1817

1817:                                             ; preds = %1805
  %1818 = getelementptr inbounds nuw i8, ptr %48, i64 8
  %1819 = getelementptr inbounds nuw i8, ptr %83, i64 8
  %1820 = getelementptr inbounds nuw i8, ptr %7, i64 664
  %1821 = getelementptr inbounds nuw i8, ptr %48, i64 16
  %1822 = getelementptr inbounds nuw i8, ptr %83, i64 16
  %1823 = getelementptr inbounds nuw i8, ptr %83, i64 24
  %1824 = getelementptr inbounds nuw i8, ptr %83, i64 32
  %1825 = getelementptr inbounds nuw i8, ptr %83, i64 40
  br label %1830

1826:                                             ; preds = %1837
  %1827 = landingpad { ptr, i32 }
          cleanup
  store ptr %1834, ptr %1813, align 8, !noalias !ID
  br label %1828

1828:                                             ; preds = %1872, %1869, %1826
  %1829 = phi { ptr, i32 } [ %1827, %1826 ], [ %1870, %1872 ], [ %1870, %1869 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %84) #ATTR
          to label %1842 unwind label %1901, !noalias !ID

1830:                                             ; preds = %1875, %1817
  %1831 = phi ptr [ %1796, %1817 ], [ %1876, %1875 ]
  %1832 = phi i64 [ %1798, %1817 ], [ %1881, %1875 ]
  %1833 = phi ptr [ %1809, %1817 ], [ %1834, %1875 ]
  %1834 = getelementptr inbounds nuw i8, ptr %1833, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %83), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !ID
  store ptr %7, ptr %48, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1818, ptr noundef nonnull align 8 dereferenceable(40) %1833, i64 40, i1 false), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1835 = load i64, ptr %1818, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1836 = icmp eq i64 %1835, 0
  br i1 %1836, label %1837, label %1839

1837:                                             ; preds = %1830
  %1838 = load ptr, ptr %1820, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %83, ptr noalias nofree noundef align 8 dereferenceable(184) %1299, ptr noundef nonnull align 8 %1838, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %1821)
          to label %1849 unwind label %1826, !noalias !ID

1839:                                             ; preds = %1830
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1819, ptr noundef nonnull align 8 dereferenceable(40) %1833, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !ID
  br label %1859

.loopexit361:                                     ; preds = %1875, %1805
  %1840 = phi i64 [ %1798, %1805 ], [ %1881, %1875 ]
  %1841 = phi ptr [ %1809, %1805 ], [ %1812, %1875 ]
  store ptr %1841, ptr %1813, align 8, !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %84)
          to label %1846 unwind label %1844, !noalias !ID

1842:                                             ; preds = %1844, %1828
  %1843 = phi { ptr, i32 } [ %1845, %1844 ], [ %1829, %1828 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %86) #ATTR, !noalias !ID
  br label %3087

1844:                                             ; preds = %1852, %.loopexit361
  %1845 = landingpad { ptr, i32 }
          cleanup
  br label %1842

1846:                                             ; preds = %.loopexit361
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !ID
  %1847 = load i64, ptr %86, align 8, !noalias !ID
  %1848 = load ptr, ptr %1806, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %86), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %93), !noalias !ID
  br label %1943

1849:                                             ; preds = %1837
  %1850 = load i64, ptr %83, align 16, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !ID
  %1851 = icmp eq i64 %1850, -1
  br i1 %1851, label %1859, label %1852

1852:                                             ; preds = %1849
  store ptr %1834, ptr %1813, align 8, !noalias !ID
  %1853 = load i64, ptr %1819, align 8, !noalias !ID
  %1854 = load ptr, ptr %1822, align 16, !noalias !ID
  %1855 = load i64, ptr %1823, align 8, !noalias !ID
  %1856 = load i64, ptr %1824, align 16, !noalias !ID
  %1857 = load i64, ptr %1825, align 8, !noalias !ID
  %1858 = getelementptr inbounds nuw i8, ptr %83, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %94, ptr noundef nonnull align 16 dereferenceable(48) %1858, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %83), !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %84)
          to label %1883 unwind label %1844, !noalias !ID

1859:                                             ; preds = %1849, %1839
  %1860 = load i64, ptr %1819, align 8, !noalias !ID
  %1861 = load ptr, ptr %1822, align 16, !noalias !ID
  %1862 = load <2 x i64>, ptr %1823, align 8, !noalias !ID
  %1863 = load i64, ptr %1825, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %83), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1864 = load i64, ptr %86, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1865 = icmp eq i64 %1832, %1864
  br i1 %1865, label %1866, label %1875

1866:                                             ; preds = %1859
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %86)
          to label %1867 unwind label %1869, !noalias !ID

1867:                                             ; preds = %1866
  %1868 = load ptr, ptr %1806, align 8, !alias.scope !ID, !noalias !ID
  br label %1875

1869:                                             ; preds = %1866
  %1870 = landingpad { ptr, i32 }
          cleanup
  store ptr %1834, ptr %1813, align 8, !noalias !ID
  %1871 = icmp ugt i64 %1860, 5
  br i1 %1871, label %1872, label %1828

1872:                                             ; preds = %1869
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1861) ]
  %1873 = shl i64 %1860, 3
  %1874 = add i64 %1873, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1861, i64 noundef %1874, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %1828

1875:                                             ; preds = %1867, %1859
  %1876 = phi ptr [ %1868, %1867 ], [ %1831, %1859 ]
  %1877 = getelementptr inbounds nuw [40 x i8], ptr %1876, i64 %1832
  store i64 %1860, ptr %1877, align 8, !noalias !ID
  %1878 = getelementptr inbounds nuw i8, ptr %1877, i64 8
  store ptr %1861, ptr %1878, align 8, !noalias !ID
  %1879 = getelementptr inbounds nuw i8, ptr %1877, i64 16
  store <2 x i64> %1862, ptr %1879, align 8, !noalias !ID
  %1880 = getelementptr inbounds nuw i8, ptr %1877, i64 32
  store i64 %1863, ptr %1880, align 8, !noalias !ID
  %1881 = add i64 %1832, 1
  store i64 %1881, ptr %1807, align 8, !alias.scope !ID, !noalias !ID
  %1882 = icmp eq ptr %1834, %1812
  br i1 %1882, label %.loopexit361, label %1830

1883:                                             ; preds = %1852
  call void @llvm.lifetime.end.p0(ptr nonnull %84), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID), !noalias !ID
  %1884 = icmp eq i64 %1832, 0
  br i1 %1884, label %.loopexit360, label %.preheader359

.preheader359:                                    ; preds = %1883, %1895
  %1885 = phi i64 [ %1887, %1895 ], [ 0, %1883 ]
  %1886 = getelementptr inbounds nuw [40 x i8], ptr %1831, i64 %1885
  %1887 = add nuw nsw i64 %1885, 1
  %1888 = load i64, ptr %1886, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1889 = icmp ugt i64 %1888, 5
  br i1 %1889, label %1890, label %1895

1890:                                             ; preds = %.preheader359
  %1891 = getelementptr i8, ptr %1886, i64 8
  %1892 = load ptr, ptr %1891, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %1893 = shl i64 %1888, 3
  %1894 = add i64 %1893, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1892, i64 noundef %1894, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %1895

1895:                                             ; preds = %1890, %.preheader359
  %1896 = icmp eq i64 %1887, %1832
  br i1 %1896, label %.loopexit360, label %.preheader359

.loopexit360:                                     ; preds = %1895, %1883
  %1897 = load i64, ptr %86, align 8, !alias.scope !ID, !noalias !ID
  %1898 = icmp eq i64 %1897, 0
  br i1 %1898, label %1932, label %1899

1899:                                             ; preds = %.loopexit360
  %1900 = mul nuw i64 %1897, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1831, i64 noundef %1900, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %1932

1901:                                             ; preds = %1903, %1828
  %1902 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

1903:                                             ; preds = %1784
  %1904 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %124) #ATTR
          to label %3089 unwind label %1901

1905:                                             ; preds = %1780
  %1906 = load i64, ptr %120, align 8, !alias.scope !ID, !noalias !ID
  %1907 = getelementptr inbounds nuw i8, ptr %120, i64 8
  %1908 = load i64, ptr %1907, align 8, !alias.scope !ID, !noalias !ID
  %1909 = getelementptr inbounds nuw i8, ptr %120, i64 16
  %1910 = load i64, ptr %1909, align 8, !alias.scope !ID, !noalias !ID
  %1911 = getelementptr inbounds nuw i8, ptr %88, i64 24
  call void @llvm.lifetime.start.p0(ptr nonnull %88), !noalias !ID
  %1912 = getelementptr inbounds nuw i8, ptr %123, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %1911, ptr noundef nonnull align 8 dereferenceable(152) %1912, i64 152, i1 false)
  %1913 = icmp ugt i64 %1906, 2
  %1914 = select i1 %1913, i64 %1910, i64 %1906
  %1915 = add i64 %1914, -1
  %1916 = select i1 %1913, i64 %1906, i64 1
  %1917 = select i1 %1913, i64 1, i64 %1910
  call void @llvm.lifetime.start.p0(ptr nonnull %89), !noalias !ID
  store i64 0, ptr %89, align 8, !noalias !ID
  %1918 = getelementptr inbounds nuw i8, ptr %89, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1918, align 8, !noalias !ID
  %1919 = getelementptr inbounds nuw i8, ptr %89, i64 16
  store i64 0, ptr %1919, align 8, !noalias !ID
  store i64 %1916, ptr %88, align 8, !noalias !ID
  %1920 = getelementptr inbounds nuw i8, ptr %88, i64 8
  store i64 %1908, ptr %1920, align 8, !noalias !ID
  %1921 = getelementptr inbounds nuw i8, ptr %88, i64 16
  store i64 %1917, ptr %1921, align 8, !noalias !ID
  %1922 = getelementptr inbounds nuw i8, ptr %88, i64 176
  store i64 0, ptr %1922, align 8, !noalias !ID
  %1923 = getelementptr inbounds nuw i8, ptr %88, i64 184
  store i64 %1915, ptr %1923, align 8, !noalias !ID
  %1924 = icmp eq i64 %1915, 0
  br i1 %1924, label %.loopexit356, label %1925

1925:                                             ; preds = %1905
  %1926 = inttoptr i64 %1908 to ptr
  %1927 = select i1 %1913, ptr %1926, ptr %1920
  %1928 = getelementptr inbounds nuw i8, ptr %87, i64 8
  %1929 = getelementptr inbounds nuw i8, ptr %87, i64 152
  br label %1957

1930:                                             ; preds = %1799
  %1931 = landingpad { ptr, i32 }
          cleanup
  br label %3089

1932:                                             ; preds = %1899, %.loopexit360
  call void @llvm.lifetime.end.p0(ptr nonnull %86), !noalias !ID
  br label %1933

1933:                                             ; preds = %1932, %1799
  %1934 = phi i64 [ %1856, %1932 ], [ %1801, %1799 ]
  %1935 = phi i64 [ %1857, %1932 ], [ %1803, %1799 ]
  %1936 = phi i64 [ %1850, %1932 ], [ %1791, %1799 ]
  %1937 = phi i64 [ %1853, %1932 ], [ %1794, %1799 ]
  %1938 = phi ptr [ %1854, %1932 ], [ %1796, %1799 ]
  %1939 = phi i64 [ %1855, %1932 ], [ %1798, %1799 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %93), !noalias !ID
  %1940 = trunc i64 %1934 to i8
  %1941 = lshr i64 %1934, 8
  %1942 = trunc nuw i64 %1941 to i56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %122, ptr noundef nonnull align 16 dereferenceable(48) %94, i64 48, i1 false), !noalias !ID
  br label %1943

1943:                                             ; preds = %1933, %1846
  %1944 = phi i56 [ 0, %1846 ], [ %1942, %1933 ]
  %1945 = phi i8 [ 0, %1846 ], [ %1940, %1933 ]
  %1946 = phi i64 [ undef, %1846 ], [ %1935, %1933 ]
  %1947 = phi i64 [ %1840, %1846 ], [ %1939, %1933 ]
  %1948 = phi ptr [ %1848, %1846 ], [ %1938, %1933 ]
  %1949 = phi i64 [ %1847, %1846 ], [ %1937, %1933 ]
  %1950 = phi i64 [ -1, %1846 ], [ %1936, %1933 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %94)
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(176) %120)
          to label %3096 unwind label %1951

1951:                                             ; preds = %1943
  %1952 = landingpad { ptr, i32 }
          cleanup
  br label %3536

1953:                                             ; preds = %1964, %1955
  %1954 = phi { ptr, i32 } [ %1956, %1955 ], [ %1974, %1964 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %89) #ATTR
          to label %3087 unwind label %1986, !noalias !ID

1955:                                             ; preds = %.loopexit356
  %1956 = landingpad { ptr, i32 }
          cleanup
  br label %1953

1957:                                             ; preds = %1984, %1925
  %1958 = phi ptr [ inttoptr (i64 8 to ptr), %1925 ], [ %1981, %1984 ]
  %1959 = phi i64 [ 0, %1925 ], [ %1960, %1984 ]
  %1960 = add nuw i64 %1959, 1
  store i64 %1960, ptr %1922, align 8, !alias.scope !ID, !noalias !ID
  %1961 = getelementptr inbounds nuw [168 x i8], ptr %1927, i64 %1959
  %1962 = load i64, ptr %1961, align 8, !noalias !ID
  %1963 = icmp eq i64 %1962, -1
  br i1 %1963, label %.loopexit356, label %1965

1964:                                             ; preds = %1973
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(192) %88)
          to label %1953 unwind label %1986, !noalias !ID

1965:                                             ; preds = %1957
  %1966 = getelementptr inbounds nuw i8, ptr %1961, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %87), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %1928, ptr noundef nonnull align 8 dereferenceable(152) %1966, i64 152, i1 false), !noalias !ID
  store i64 %1962, ptr %87, align 8, !noalias !ID
  %1967 = load i8, ptr %1929, align 8, !range !ID, !noalias !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %1968 = load i64, ptr %89, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %1969 = icmp eq i64 %1959, %1968
  br i1 %1969, label %1970, label %1980

1970:                                             ; preds = %1965
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %89)
          to label %1971 unwind label %1973, !noalias !ID

1971:                                             ; preds = %1970
  %1972 = load ptr, ptr %1918, align 8, !alias.scope !ID, !noalias !ID
  br label %1980

1973:                                             ; preds = %1970
  %1974 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(160) %87) #ATTR
          to label %1964 unwind label %1975, !noalias !ID

1975:                                             ; preds = %1973
  %1976 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

.loopexit356:                                     ; preds = %1980, %1984, %1957, %1905
  %1977 = phi i64 [ 0, %1905 ], [ %1960, %1980 ], [ %1915, %1984 ], [ %1959, %1957 ]
  %1978 = phi ptr [ inttoptr (i64 8 to ptr), %1905 ], [ %1981, %1980 ], [ %1981, %1984 ], [ %1958, %1957 ]
  %1979 = phi i8 [ 0, %1905 ], [ 1, %1980 ], [ 0, %1984 ], [ 0, %1957 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::IntoIter<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(192) %88)
          to label %1988 unwind label %1955, !noalias !ID

1980:                                             ; preds = %1971, %1965
  %1981 = phi ptr [ %1972, %1971 ], [ %1958, %1965 ]
  %1982 = getelementptr inbounds nuw [160 x i8], ptr %1981, i64 %1959
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(160) %1982, ptr noundef nonnull readonly align 8 dereferenceable(160) %87, i64 160, i1 false), !noalias !ID
  store i64 %1960, ptr %1919, align 8, !alias.scope !ID, !noalias !ID
  %1983 = trunc nuw i8 %1967 to i1
  call void @llvm.lifetime.end.p0(ptr nonnull %87), !noalias !ID
  br i1 %1983, label %.loopexit356, label %1984

1984:                                             ; preds = %1980
  %1985 = icmp eq i64 %1960, %1915
  br i1 %1985, label %.loopexit356, label %1957

1986:                                             ; preds = %1964, %1953
  %1987 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

1988:                                             ; preds = %.loopexit356
  call void @llvm.lifetime.end.p0(ptr nonnull %88), !noalias !ID
  %1989 = load i64, ptr %89, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %89), !noalias !ID
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1978) ]
  %1990 = icmp eq i64 %1977, 0
  br i1 %1990, label %.loopexit355, label %iter.check

iter.check:                                       ; preds = %1988
  %min.iters.check = icmp ult i64 %1977, 8
  br i1 %min.iters.check, label %.preheader354.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check2449 = icmp ult i64 %1977, 32
  br i1 %min.iters.check2449, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %1977, 24
  %n.vec = and i64 %1977, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1991, %vector.body ]
  %vec.phi2450 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1992, %vector.body ]
  %vec.phi2451 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1993, %vector.body ]
  %vec.phi2452 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %1994, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %vec.ind
  %wide.gep2453 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %step.add
  %wide.gep2454 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %step.add.2
  %wide.gep2455 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %step.add.3
  %wide.gep2456 = getelementptr i8, <8 x ptr> %wide.gep, i64 64
  %wide.gep2457 = getelementptr i8, <8 x ptr> %wide.gep2453, i64 64
  %wide.gep2458 = getelementptr i8, <8 x ptr> %wide.gep2454, i64 64
  %wide.gep2459 = getelementptr i8, <8 x ptr> %wide.gep2455, i64 64
  %wide.masked.gather = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2456, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %wide.masked.gather2460 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2457, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %wide.masked.gather2461 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2458, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %wide.masked.gather2462 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2459, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %1991 = add <8 x i64> %wide.masked.gather, %vec.phi
  %1992 = add <8 x i64> %wide.masked.gather2460, %vec.phi2450
  %1993 = add <8 x i64> %wide.masked.gather2461, %vec.phi2451
  %1994 = add <8 x i64> %wide.masked.gather2462, %vec.phi2452
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %1995 = icmp eq i64 %index.next, %n.vec
  br i1 %1995, label %middle.block, label %vector.body, !llvm.loop !ID

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %1992, %1991
  %bin.rdx2463 = add <8 x i64> %1993, %bin.rdx
  %bin.rdx2464 = add <8 x i64> %1994, %bin.rdx2463
  %1996 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx2464)
  %cmp.n = icmp eq i64 %1977, %n.vec
  br i1 %cmp.n, label %.loopexit355, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader354.preheader, label %vec.epilog.ph, !prof !ID

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %1996, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec2466 = and i64 %1977, -8
  %1997 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index2467 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next2473, %vec.epilog.vector.body ]
  %vec.ind2468 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next2474, %vec.epilog.vector.body ]
  %vec.phi2469 = phi <8 x i64> [ %1997, %vec.epilog.ph ], [ %1998, %vec.epilog.vector.body ]
  %wide.gep2470 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %vec.ind2468
  %wide.gep2471 = getelementptr i8, <8 x ptr> %wide.gep2470, i64 64
  %wide.masked.gather2472 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2471, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %1998 = add <8 x i64> %wide.masked.gather2472, %vec.phi2469
  %index.next2473 = add nuw i64 %index2467, 8
  %vec.ind.next2474 = add nuw <8 x i64> %vec.ind2468, splat (i64 8)
  %1999 = icmp eq i64 %index.next2473, %n.vec2466
  br i1 %1999, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !ID

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %2000 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %1998)
  %cmp.n2475 = icmp eq i64 %1977, %n.vec2466
  br i1 %cmp.n2475, label %.loopexit355, label %.preheader354.preheader

.preheader354.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph2857 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec2466, %vec.epilog.middle.block ]
  %.ph2858 = phi i64 [ 0, %iter.check ], [ %1996, %vec.epilog.iter.check ], [ %2000, %vec.epilog.middle.block ]
  br label %.preheader354

.preheader354:                                    ; preds = %.preheader354.preheader, %.preheader354
  %2001 = phi i64 [ %2008, %.preheader354 ], [ %.ph2857, %.preheader354.preheader ]
  %2002 = phi i64 [ %2007, %.preheader354 ], [ %.ph2858, %.preheader354.preheader ]
  %2003 = getelementptr inbounds nuw [160 x i8], ptr %1978, i64 %2001
  %2004 = getelementptr i8, ptr %2003, i64 64
  %2005 = load i64, ptr %2004, align 8, !noalias !ID, !noundef !ID
  %2006 = icmp ult i64 %2005, 288230376151711744
  call void @llvm.assume(i1 %2006)
  %2007 = add i64 %2005, %2002
  %2008 = add nuw i64 %2001, 1
  %2009 = icmp eq i64 %2008, %1977
  br i1 %2009, label %.loopexit355, label %.preheader354, !llvm.loop !ID

.loopexit355:                                     ; preds = %.preheader354, %middle.block, %vec.epilog.middle.block, %1988
  %2010 = phi i64 [ 0, %1988 ], [ %2000, %vec.epilog.middle.block ], [ %1996, %middle.block ], [ %2007, %.preheader354 ]
  %2011 = trunc nuw i8 %1979 to i1
  call void @llvm.lifetime.start.p0(ptr nonnull %92)
  call void @llvm.lifetime.start.p0(ptr nonnull %91), !noalias !ID
  store i64 %1989, ptr %91, align 8, !noalias !ID
  %2012 = getelementptr inbounds nuw i8, ptr %91, i64 8
  store ptr %1978, ptr %2012, align 8, !noalias !ID
  %2013 = getelementptr inbounds nuw i8, ptr %91, i64 16
  store i64 %1977, ptr %2013, align 8, !noalias !ID
  %2014 = getelementptr inbounds nuw i8, ptr %91, i64 24
  store i8 %1979, ptr %2014, align 8, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.lifetime.start.p0(ptr nonnull %82), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %81), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %80), !noalias !ID
  store i64 0, ptr %80, align 8, !noalias !ID
  %2015 = getelementptr inbounds nuw i8, ptr %80, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %2015, align 8, !noalias !ID
  %2016 = getelementptr inbounds nuw i8, ptr %80, i64 16
  store i64 0, ptr %2016, align 8, !noalias !ID
  %2017 = getelementptr inbounds nuw i8, ptr %121, i64 16
  %2018 = load i64, ptr %2017, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2019 = icmp ult i64 %2018, 230584300921369396
  call void @llvm.assume(i1 %2019)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %81, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %80, i64 noundef %2018)
          to label %2020 unwind label %3025, !noalias !ID

2020:                                             ; preds = %.loopexit355
  call void @llvm.lifetime.end.p0(ptr nonnull %80), !noalias !ID
  %2021 = load i64, ptr %81, align 16, !range !ID, !noalias !ID, !noundef !ID
  %2022 = icmp eq i64 %2021, -1
  %2023 = getelementptr inbounds nuw i8, ptr %81, i64 8
  %2024 = load i64, ptr %2023, align 8, !noalias !ID
  %2025 = getelementptr inbounds nuw i8, ptr %81, i64 16
  %2026 = load ptr, ptr %2025, align 16, !noalias !ID
  %2027 = getelementptr inbounds nuw i8, ptr %81, i64 24
  %2028 = load i64, ptr %2027, align 8, !noalias !ID
  br i1 %2022, label %2037, label %2029

2029:                                             ; preds = %2020
  %2030 = getelementptr inbounds nuw i8, ptr %81, i64 32
  %2031 = load i8, ptr %2030, align 16, !noalias !ID
  %2032 = getelementptr inbounds nuw i8, ptr %81, i64 33
  %2033 = load i56, ptr %2032, align 1, !noalias !ID
  %2034 = getelementptr inbounds nuw i8, ptr %81, i64 40
  %2035 = load i64, ptr %2034, align 8, !noalias !ID
  %2036 = getelementptr inbounds nuw i8, ptr %81, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %92, ptr noundef nonnull align 16 dereferenceable(48) %2036, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %81), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %124)
          to label %2995 unwind label %2993

2037:                                             ; preds = %2020
  call void @llvm.lifetime.end.p0(ptr nonnull %81), !noalias !ID
  store i64 %2024, ptr %82, align 8, !noalias !ID
  %2038 = getelementptr inbounds nuw i8, ptr %82, i64 8
  store ptr %2026, ptr %2038, align 8, !noalias !ID
  %2039 = getelementptr inbounds nuw i8, ptr %82, i64 16
  store i64 %2028, ptr %2039, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %79), !noalias !ID
  %2040 = getelementptr inbounds nuw i8, ptr %121, i64 8
  %2041 = load ptr, ptr %2040, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %2042 = load i64, ptr %121, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2043 = getelementptr inbounds nuw [40 x i8], ptr %2041, i64 %2018
  store ptr %2041, ptr %79, align 8, !noalias !ID
  %2044 = getelementptr inbounds nuw i8, ptr %79, i64 16
  store i64 %2042, ptr %2044, align 8, !noalias !ID
  %2045 = getelementptr inbounds nuw i8, ptr %79, i64 8
  store ptr %2041, ptr %2045, align 8, !noalias !ID
  %2046 = getelementptr inbounds nuw i8, ptr %79, i64 24
  store ptr %2043, ptr %2046, align 8, !noalias !ID
  %2047 = load ptr, ptr %1306, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2048 = icmp eq ptr %2047, null
  br i1 %2048, label %2052, label %2049

2049:                                             ; preds = %2037
  %2050 = atomicrmw add ptr %2047, i64 1 monotonic, align 8, !noalias !ID
  %2051 = icmp slt i64 %2050, 0
  br i1 %2051, label %2162, label %2154

2052:                                             ; preds = %2037
  call void @llvm.lifetime.start.p0(ptr nonnull %78), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %78, ptr noundef nonnull align 8 dereferenceable(32) %79, i64 32, i1 false), !noalias !ID
  %2053 = getelementptr inbounds nuw i8, ptr %78, i64 24
  %2054 = load ptr, ptr %2053, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %2055 = getelementptr inbounds nuw i8, ptr %78, i64 8
  %2056 = load ptr, ptr %2055, align 8, !alias.scope !ID, !noalias !ID
  %2057 = icmp eq ptr %2056, %2054
  br i1 %2057, label %.loopexit319, label %2058

2058:                                             ; preds = %2052
  %2059 = getelementptr inbounds nuw i8, ptr %76, i64 8
  %2060 = getelementptr inbounds nuw i8, ptr %77, i64 8
  %2061 = getelementptr inbounds nuw i8, ptr %7, i64 664
  %2062 = getelementptr inbounds nuw i8, ptr %76, i64 16
  %2063 = getelementptr inbounds nuw i8, ptr %77, i64 16
  %2064 = getelementptr inbounds nuw i8, ptr %77, i64 24
  %2065 = getelementptr inbounds nuw i8, ptr %77, i64 32
  %2066 = getelementptr inbounds nuw i8, ptr %77, i64 33
  %2067 = getelementptr inbounds nuw i8, ptr %77, i64 40
  br label %2072

2068:                                             ; preds = %2079
  %2069 = landingpad { ptr, i32 }
          cleanup
  store ptr %2076, ptr %2055, align 8, !noalias !ID
  br label %2070

2070:                                             ; preds = %2128, %2125, %2068
  %2071 = phi { ptr, i32 } [ %2069, %2068 ], [ %2126, %2128 ], [ %2126, %2125 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %78) #ATTR
          to label %2986 unwind label %2152, !noalias !ID

2072:                                             ; preds = %2131, %2058
  %2073 = phi ptr [ %2026, %2058 ], [ %2132, %2131 ]
  %2074 = phi i64 [ %2028, %2058 ], [ %2139, %2131 ]
  %2075 = phi ptr [ %2056, %2058 ], [ %2076, %2131 ]
  %2076 = getelementptr inbounds nuw i8, ptr %2075, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %76), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %2059, ptr noundef nonnull align 8 dereferenceable(40) %2075, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %77), !noalias !ID
  store ptr %7, ptr %76, align 8, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2077 = load i64, ptr %2059, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2078 = icmp eq i64 %2077, 0
  br i1 %2078, label %2079, label %2081

2079:                                             ; preds = %2072
  %2080 = load ptr, ptr %2061, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %77, ptr noalias nofree noundef align 8 dereferenceable(184) %1299, ptr noundef nonnull align 8 %2080, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %2062)
          to label %2102 unwind label %2068, !noalias !ID

2081:                                             ; preds = %2072
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %2060, ptr noundef nonnull align 8 dereferenceable(40) %2075, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %76), !noalias !ID
  br label %2113

.loopexit319:                                     ; preds = %2131, %2052
  %2082 = phi i64 [ %2028, %2052 ], [ %2139, %2131 ]
  %2083 = phi ptr [ %2056, %2052 ], [ %2076, %2131 ]
  store ptr %2083, ptr %2055, align 8, !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %78)
          to label %2088 unwind label %2084, !noalias !ID

2084:                                             ; preds = %2950, %2312, %2105, %.loopexit319
  %2085 = phi i8 [ %2922, %2950 ], [ 0, %2312 ], [ 1, %2105 ], [ 1, %.loopexit319 ]
  %2086 = phi i8 [ 0, %2950 ], [ 0, %2312 ], [ 1, %2105 ], [ 1, %.loopexit319 ]
  %2087 = landingpad { ptr, i32 }
          cleanup
  br i1 %2048, label %2091, label %2982

2088:                                             ; preds = %.loopexit319
  call void @llvm.lifetime.end.p0(ptr nonnull %78), !noalias !ID
  %2089 = load i64, ptr %82, align 8, !noalias !ID
  %2090 = load ptr, ptr %2038, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !ID
  br label %2995

2091:                                             ; preds = %2982, %2096, %2084
  %2092 = phi i8 [ %2097, %2096 ], [ %2985, %2982 ], [ %2085, %2084 ]
  %2093 = phi i8 [ %2098, %2096 ], [ %2984, %2982 ], [ %2086, %2084 ]
  %2094 = phi { ptr, i32 } [ %2099, %2096 ], [ %2983, %2982 ], [ %2087, %2084 ]
  %2095 = trunc nuw i8 %2092 to i1
  br i1 %2095, label %2986, label %2989

2096:                                             ; preds = %2954, %2313
  %2097 = phi i8 [ %2150, %2954 ], [ 0, %2313 ]
  %2098 = phi i8 [ %2151, %2954 ], [ 0, %2313 ]
  %2099 = landingpad { ptr, i32 }
          cleanup
  br label %2091

2100:                                             ; preds = %2973, %.loopexit318, %2952
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !ID
  %2101 = trunc nuw i8 %2151 to i1
  br i1 %2101, label %2995, label %3029

2102:                                             ; preds = %2079
  %2103 = load i64, ptr %77, align 16, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %76), !noalias !ID
  %2104 = icmp eq i64 %2103, -1
  br i1 %2104, label %2113, label %2105

2105:                                             ; preds = %2102
  store ptr %2076, ptr %2055, align 8, !noalias !ID
  %2106 = load i64, ptr %2060, align 8, !noalias !ID
  %2107 = load ptr, ptr %2063, align 16, !noalias !ID
  %2108 = load i64, ptr %2064, align 8, !noalias !ID
  %2109 = load i8, ptr %2065, align 16, !noalias !ID
  %2110 = load i56, ptr %2066, align 1, !noalias !ID
  %2111 = load i64, ptr %2067, align 8, !noalias !ID
  %2112 = getelementptr inbounds nuw i8, ptr %77, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %92, ptr noundef nonnull align 16 dereferenceable(48) %2112, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %77), !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %78)
          to label %2141 unwind label %2084, !noalias !ID

2113:                                             ; preds = %2102, %2081
  %2114 = load i64, ptr %2060, align 8, !noalias !ID
  %2115 = load ptr, ptr %2063, align 16, !noalias !ID
  %2116 = load i64, ptr %2064, align 8, !noalias !ID
  %2117 = load i8, ptr %2065, align 16, !noalias !ID
  %2118 = load i56, ptr %2066, align 1, !noalias !ID
  %2119 = load i64, ptr %2067, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %77), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2120 = load i64, ptr %82, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2121 = icmp eq i64 %2074, %2120
  br i1 %2121, label %2122, label %2131

2122:                                             ; preds = %2113
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %82)
          to label %2123 unwind label %2125, !noalias !ID

2123:                                             ; preds = %2122
  %2124 = load ptr, ptr %2038, align 8, !alias.scope !ID, !noalias !ID
  br label %2131

2125:                                             ; preds = %2122
  %2126 = landingpad { ptr, i32 }
          cleanup
  store ptr %2076, ptr %2055, align 8, !noalias !ID
  %2127 = icmp ugt i64 %2114, 5
  br i1 %2127, label %2128, label %2070

2128:                                             ; preds = %2125
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2115) ]
  %2129 = shl i64 %2114, 3
  %2130 = add i64 %2129, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2115, i64 noundef %2130, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %2070

2131:                                             ; preds = %2123, %2113
  %2132 = phi ptr [ %2124, %2123 ], [ %2073, %2113 ]
  %2133 = getelementptr inbounds nuw [40 x i8], ptr %2132, i64 %2074
  store i64 %2114, ptr %2133, align 8, !noalias !ID
  %2134 = getelementptr inbounds nuw i8, ptr %2133, i64 8
  store ptr %2115, ptr %2134, align 8, !noalias !ID
  %2135 = getelementptr inbounds nuw i8, ptr %2133, i64 16
  store i64 %2116, ptr %2135, align 8, !noalias !ID
  %2136 = getelementptr inbounds nuw i8, ptr %2133, i64 24
  store i8 %2117, ptr %2136, align 8, !noalias !ID
  %2137 = getelementptr inbounds nuw i8, ptr %2133, i64 25
  store i56 %2118, ptr %2137, align 1, !noalias !ID
  %2138 = getelementptr inbounds nuw i8, ptr %2133, i64 32
  store i64 %2119, ptr %2138, align 8, !noalias !ID
  %2139 = add i64 %2074, 1
  store i64 %2139, ptr %2039, align 8, !alias.scope !ID, !noalias !ID
  %2140 = icmp eq ptr %2076, %2054
  br i1 %2140, label %.loopexit319, label %2072

2141:                                             ; preds = %2105
  call void @llvm.lifetime.end.p0(ptr nonnull %78), !noalias !ID
  br label %2142

2142:                                             ; preds = %2951, %2141
  %2143 = phi i56 [ %2110, %2141 ], [ %2915, %2951 ]
  %2144 = phi i64 [ %2111, %2141 ], [ %2916, %2951 ]
  %2145 = phi i64 [ %2106, %2141 ], [ %2917, %2951 ]
  %2146 = phi ptr [ %2107, %2141 ], [ %2918, %2951 ]
  %2147 = phi i64 [ %2108, %2141 ], [ %2919, %2951 ]
  %2148 = phi i8 [ %2109, %2141 ], [ %2920, %2951 ]
  %2149 = phi i64 [ %2103, %2141 ], [ %2921, %2951 ]
  %2150 = phi i8 [ 1, %2141 ], [ %2922, %2951 ]
  %2151 = phi i8 [ 1, %2141 ], [ 0, %2951 ]
  br i1 %2048, label %2952, label %2954

2152:                                             ; preds = %3027, %3025, %2982, %2365, %2280, %2261, %2070
  %2153 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

2154:                                             ; preds = %2049
  %2155 = load ptr, ptr %1306, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %75), !noalias !ID
  store ptr %2155, ptr %75, align 8, !noalias !ID
  %2156 = getelementptr inbounds nuw i8, ptr %2155, i64 16
  %2157 = load i64, ptr %2156, align 8, !noalias !ID
  %2158 = icmp eq i64 %2157, -1
  %2159 = getelementptr inbounds nuw i8, ptr %2155, i64 40
  %2160 = load i64, ptr %2159, align 8, !noalias !ID
  %2161 = icmp ne i64 %2160, -1
  br i1 %2161, label %2231, label %2170

2162:                                             ; preds = %2049
  call void @llvm.trap()
  unreachable

2163:                                             ; preds = %2277, %2273
  %2164 = icmp ult i64 %1977, 57646075230342349
  call void @llvm.assume(i1 %2164)
  %2165 = mul nuw nsw i64 %1977, 160
  %2166 = getelementptr inbounds nuw i8, ptr %1978, i64 %2165
  call void @llvm.lifetime.start.p0(ptr nonnull %74), !noalias !ID
  store ptr %1978, ptr %74, align 8, !noalias !ID
  %2167 = getelementptr inbounds nuw i8, ptr %74, i64 8
  store ptr %1978, ptr %2167, align 8, !noalias !ID
  %2168 = getelementptr inbounds nuw i8, ptr %74, i64 16
  store i64 %1989, ptr %2168, align 8, !noalias !ID
  %2169 = getelementptr inbounds nuw i8, ptr %74, i64 24
  store ptr %2166, ptr %2169, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %73)
  br label %2177

2170:                                             ; preds = %2154
  %2171 = icmp ult i64 %1977, 57646075230342349
  call void @llvm.assume(i1 %2171)
  %2172 = mul nuw nsw i64 %1977, 160
  %2173 = getelementptr inbounds nuw i8, ptr %1978, i64 %2172
  call void @llvm.lifetime.start.p0(ptr nonnull %74), !noalias !ID
  store ptr %1978, ptr %74, align 8, !noalias !ID
  %2174 = getelementptr inbounds nuw i8, ptr %74, i64 8
  store ptr %1978, ptr %2174, align 8, !noalias !ID
  %2175 = getelementptr inbounds nuw i8, ptr %74, i64 16
  store i64 %1989, ptr %2175, align 8, !noalias !ID
  %2176 = getelementptr inbounds nuw i8, ptr %74, i64 24
  store ptr %2173, ptr %2176, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %73)
  br i1 %1990, label %.loopexit352, label %2177

2177:                                             ; preds = %2170, %2163
  %2178 = phi ptr [ %2167, %2163 ], [ %2174, %2170 ]
  %2179 = phi ptr [ %2166, %2163 ], [ %2173, %2170 ]
  %2180 = getelementptr inbounds nuw i8, ptr %72, i64 8
  %2181 = getelementptr inbounds nuw i8, ptr %72, i64 24
  %2182 = getelementptr inbounds nuw i8, ptr %72, i64 32
  %2183 = getelementptr inbounds nuw i8, ptr %72, i64 40
  %2184 = getelementptr inbounds nuw i8, ptr %72, i64 16
  %2185 = getelementptr inbounds nuw i8, ptr %71, i64 16
  %2186 = getelementptr inbounds nuw i8, ptr %71, i64 8
  %2187 = getelementptr inbounds nuw i8, ptr %71, i64 24
  %2188 = getelementptr inbounds nuw i8, ptr %72, i64 96
  %2189 = getelementptr inbounds nuw i8, ptr %72, i64 48
  %2190 = getelementptr inbounds nuw i8, ptr %72, i64 56
  %2191 = getelementptr inbounds nuw i8, ptr %72, i64 64
  %2192 = getelementptr inbounds nuw i8, ptr %2155, i64 80
  %2193 = getelementptr inbounds nuw i8, ptr %43, i64 1
  %2194 = getelementptr inbounds nuw i8, ptr %43, i64 8
  %2195 = getelementptr inbounds nuw i8, ptr %43, i64 16
  %2196 = getelementptr inbounds nuw i8, ptr %2155, i64 296
  %2197 = getelementptr inbounds nuw i8, ptr %2155, i64 272
  %2198 = getelementptr inbounds nuw i8, ptr %7, i64 1048
  %2199 = getelementptr inbounds nuw i8, ptr %7, i64 1056
  %2200 = getelementptr inbounds nuw i8, ptr %2155, i64 104
  %2201 = getelementptr inbounds nuw i8, ptr %7, i64 632
  %2202 = getelementptr inbounds nuw i8, ptr %7, i64 1228
  %2203 = getelementptr inbounds nuw i8, ptr %62, i64 8
  %2204 = getelementptr inbounds nuw i8, ptr %59, i64 8
  %2205 = getelementptr inbounds nuw i8, ptr %60, i64 16
  %2206 = getelementptr inbounds nuw i8, ptr %58, i64 8
  %2207 = getelementptr inbounds nuw i8, ptr %40, i64 1
  %2208 = getelementptr inbounds nuw i8, ptr %40, i64 8
  %2209 = getelementptr inbounds nuw i8, ptr %40, i64 16
  %2210 = getelementptr inbounds nuw i8, ptr %56, i64 8
  %2211 = getelementptr inbounds nuw i8, ptr %37, i64 1
  %2212 = getelementptr inbounds nuw i8, ptr %37, i64 8
  %2213 = getelementptr inbounds nuw i8, ptr %37, i64 16
  %2214 = getelementptr inbounds nuw i8, ptr %52, i64 8
  %2215 = getelementptr inbounds nuw i8, ptr %41, i64 1
  %2216 = getelementptr inbounds nuw i8, ptr %41, i64 8
  %2217 = getelementptr inbounds nuw i8, ptr %41, i64 16
  %2218 = getelementptr inbounds nuw i8, ptr %54, i64 8
  %2219 = getelementptr inbounds nuw i8, ptr %50, i64 8
  %2220 = getelementptr inbounds nuw i8, ptr %63, i64 8
  %2221 = getelementptr inbounds nuw i8, ptr %64, i64 8
  %2222 = getelementptr inbounds nuw i8, ptr %7, i64 664
  %2223 = getelementptr inbounds nuw i8, ptr %63, i64 16
  %2224 = getelementptr inbounds nuw i8, ptr %64, i64 16
  %2225 = getelementptr inbounds nuw i8, ptr %64, i64 24
  %2226 = getelementptr inbounds nuw i8, ptr %64, i64 32
  %2227 = getelementptr inbounds nuw i8, ptr %64, i64 33
  %2228 = getelementptr inbounds nuw i8, ptr %72, i64 88
  %2229 = getelementptr inbounds nuw i8, ptr %72, i64 120
  %2230 = getelementptr inbounds nuw i8, ptr %64, i64 40
  br label %2281

2231:                                             ; preds = %2154
  br i1 %1990, label %2242, label %iter.check2514

iter.check2514:                                   ; preds = %2231
  %min.iters.check2477 = icmp ult i64 %1977, 8
  br i1 %min.iters.check2477, label %.preheader353.preheader, label %vector.main.loop.iter.check2478

vector.main.loop.iter.check2478:                  ; preds = %iter.check2514
  %min.iters.check2479 = icmp ult i64 %1977, 32
  br i1 %min.iters.check2479, label %vec.epilog.ph2518, label %vector.ph2480

vector.ph2480:                                    ; preds = %vector.main.loop.iter.check2478
  %n.mod.vf2481 = and i64 %1977, 24
  %n.vec2482 = and i64 %1977, -32
  br label %vector.body2483

vector.body2483:                                  ; preds = %vector.body2483, %vector.ph2480
  %index2484 = phi i64 [ 0, %vector.ph2480 ], [ %index.next2505, %vector.body2483 ]
  %vec.ind2485 = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph2480 ], [ %vec.ind.next2506, %vector.body2483 ]
  %vec.phi2486 = phi <8 x i64> [ zeroinitializer, %vector.ph2480 ], [ %2232, %vector.body2483 ]
  %vec.phi2487 = phi <8 x i64> [ zeroinitializer, %vector.ph2480 ], [ %2233, %vector.body2483 ]
  %vec.phi2488 = phi <8 x i64> [ zeroinitializer, %vector.ph2480 ], [ %2234, %vector.body2483 ]
  %vec.phi2489 = phi <8 x i64> [ zeroinitializer, %vector.ph2480 ], [ %2235, %vector.body2483 ]
  %step.add2490 = add nuw <8 x i64> %vec.ind2485, splat (i64 8)
  %step.add.22491 = add nuw <8 x i64> %vec.ind2485, splat (i64 16)
  %step.add.32492 = add nuw <8 x i64> %vec.ind2485, splat (i64 24)
  %wide.gep2493 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %vec.ind2485
  %wide.gep2494 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %step.add2490
  %wide.gep2495 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %step.add.22491
  %wide.gep2496 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %step.add.32492
  %wide.gep2497 = getelementptr i8, <8 x ptr> %wide.gep2493, i64 16
  %wide.gep2498 = getelementptr i8, <8 x ptr> %wide.gep2494, i64 16
  %wide.gep2499 = getelementptr i8, <8 x ptr> %wide.gep2495, i64 16
  %wide.gep2500 = getelementptr i8, <8 x ptr> %wide.gep2496, i64 16
  %wide.masked.gather2501 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2497, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %wide.masked.gather2502 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2498, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %wide.masked.gather2503 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2499, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %wide.masked.gather2504 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2500, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %2232 = add <8 x i64> %wide.masked.gather2501, %vec.phi2486
  %2233 = add <8 x i64> %wide.masked.gather2502, %vec.phi2487
  %2234 = add <8 x i64> %wide.masked.gather2503, %vec.phi2488
  %2235 = add <8 x i64> %wide.masked.gather2504, %vec.phi2489
  %index.next2505 = add nuw i64 %index2484, 32
  %vec.ind.next2506 = add nuw <8 x i64> %vec.ind2485, splat (i64 32)
  %2236 = icmp eq i64 %index.next2505, %n.vec2482
  br i1 %2236, label %middle.block2507, label %vector.body2483, !llvm.loop !ID

middle.block2507:                                 ; preds = %vector.body2483
  %bin.rdx2508 = add <8 x i64> %2233, %2232
  %bin.rdx2509 = add <8 x i64> %2234, %bin.rdx2508
  %bin.rdx2510 = add <8 x i64> %2235, %bin.rdx2509
  %2237 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx2510)
  %cmp.n2511 = icmp eq i64 %1977, %n.vec2482
  br i1 %cmp.n2511, label %.loopexit2537, label %vec.epilog.iter.check2516

vec.epilog.iter.check2516:                        ; preds = %middle.block2507
  %min.epilog.iters.check2517 = icmp eq i64 %n.mod.vf2481, 0
  br i1 %min.epilog.iters.check2517, label %.preheader353.preheader, label %vec.epilog.ph2518, !prof !ID

vec.epilog.ph2518:                                ; preds = %vector.main.loop.iter.check2478, %vec.epilog.iter.check2516
  %vec.epilog.resume.val2512 = phi i64 [ %n.vec2482, %vec.epilog.iter.check2516 ], [ 0, %vector.main.loop.iter.check2478 ]
  %bc.merge.rdx2513 = phi i64 [ %2237, %vec.epilog.iter.check2516 ], [ 0, %vector.main.loop.iter.check2478 ]
  %n.vec2520 = and i64 %1977, -8
  %2238 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx2513, i64 0
  %broadcast.splatinsert2521 = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val2512, i64 0
  %broadcast.splat2522 = shufflevector <8 x i64> %broadcast.splatinsert2521, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction2523 = or disjoint <8 x i64> %broadcast.splat2522, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body2524

vec.epilog.vector.body2524:                       ; preds = %vec.epilog.vector.body2524, %vec.epilog.ph2518
  %index2525 = phi i64 [ %vec.epilog.resume.val2512, %vec.epilog.ph2518 ], [ %index.next2531, %vec.epilog.vector.body2524 ]
  %vec.ind2526 = phi <8 x i64> [ %induction2523, %vec.epilog.ph2518 ], [ %vec.ind.next2532, %vec.epilog.vector.body2524 ]
  %vec.phi2527 = phi <8 x i64> [ %2238, %vec.epilog.ph2518 ], [ %2239, %vec.epilog.vector.body2524 ]
  %wide.gep2528 = getelementptr inbounds nuw [160 x i8], ptr %1978, <8 x i64> %vec.ind2526
  %wide.gep2529 = getelementptr i8, <8 x ptr> %wide.gep2528, i64 16
  %wide.masked.gather2530 = call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep2529, <8 x i1> splat (i1 true), <8 x i64> poison), !noalias !ID
  %2239 = add <8 x i64> %wide.masked.gather2530, %vec.phi2527
  %index.next2531 = add nuw i64 %index2525, 8
  %vec.ind.next2532 = add nuw <8 x i64> %vec.ind2526, splat (i64 8)
  %2240 = icmp eq i64 %index.next2531, %n.vec2520
  br i1 %2240, label %vec.epilog.middle.block2533, label %vec.epilog.vector.body2524, !llvm.loop !ID

vec.epilog.middle.block2533:                      ; preds = %vec.epilog.vector.body2524
  %2241 = call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %2239)
  %cmp.n2534 = icmp eq i64 %1977, %n.vec2520
  br i1 %cmp.n2534, label %.loopexit2537, label %.preheader353.preheader

.preheader353.preheader:                          ; preds = %iter.check2514, %vec.epilog.iter.check2516, %vec.epilog.middle.block2533
  %.ph2849 = phi i64 [ 0, %iter.check2514 ], [ %n.vec2482, %vec.epilog.iter.check2516 ], [ %n.vec2520, %vec.epilog.middle.block2533 ]
  %.ph2850 = phi i64 [ 0, %iter.check2514 ], [ %2237, %vec.epilog.iter.check2516 ], [ %2241, %vec.epilog.middle.block2533 ]
  br label %.preheader353

2242:                                             ; preds = %2231
  call void @llvm.lifetime.start.p0(ptr nonnull %74), !noalias !ID
  store ptr %1978, ptr %74, align 8, !noalias !ID
  %2243 = getelementptr inbounds nuw i8, ptr %74, i64 8
  store ptr %1978, ptr %2243, align 8, !noalias !ID
  %2244 = getelementptr inbounds nuw i8, ptr %74, i64 16
  store i64 %1989, ptr %2244, align 8, !noalias !ID
  %2245 = getelementptr inbounds nuw i8, ptr %74, i64 24
  store ptr %1978, ptr %2245, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %73)
  br label %.loopexit352

.preheader353:                                    ; preds = %.preheader353.preheader, %.preheader353
  %2246 = phi i64 [ %2253, %.preheader353 ], [ %.ph2849, %.preheader353.preheader ]
  %2247 = phi i64 [ %2252, %.preheader353 ], [ %.ph2850, %.preheader353.preheader ]
  %2248 = getelementptr inbounds nuw [160 x i8], ptr %1978, i64 %2246
  %2249 = getelementptr i8, ptr %2248, i64 16
  %2250 = load i64, ptr %2249, align 8, !noalias !ID, !noundef !ID
  %2251 = icmp ult i64 %2250, 104811045873349726
  call void @llvm.assume(i1 %2251), !noalias !ID
  %2252 = add i64 %2250, %2247
  %2253 = add nuw i64 %2246, 1
  %2254 = icmp eq i64 %2253, %1977
  br i1 %2254, label %.loopexit2537, label %.preheader353, !llvm.loop !ID

2255:                                             ; preds = %2280, %2262
  %2256 = phi i8 [ %2263, %2262 ], [ %2368, %2280 ]
  %2257 = phi i8 [ %2264, %2262 ], [ 0, %2280 ]
  %2258 = phi { ptr, i32 } [ %2265, %2262 ], [ %2369, %2280 ]
  %2259 = atomicrmw sub ptr %2155, i64 1 release, align 8, !noalias !ID
  %2260 = icmp eq i64 %2259, 1
  br i1 %2260, label %2261, label %2982

2261:                                             ; preds = %2255
  fence acquire, !noalias !ID
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %75) #ATTR
          to label %2982 unwind label %2152, !noalias !ID

2262:                                             ; preds = %2946, %.loopexit352, %2277, %2272
  %2263 = phi i8 [ %2922, %2946 ], [ 1, %2277 ], [ 1, %.loopexit352 ], [ 1, %2272 ]
  %2264 = phi i8 [ 0, %2946 ], [ 1, %2277 ], [ 0, %.loopexit352 ], [ 1, %2272 ]
  %2265 = landingpad { ptr, i32 }
          cleanup
  br label %2255

.loopexit2537:                                    ; preds = %.preheader353, %vec.epilog.middle.block2533, %middle.block2507
  %.lcssa2304 = phi i64 [ %2241, %vec.epilog.middle.block2533 ], [ %2237, %middle.block2507 ], [ %2252, %.preheader353 ]
  %2266 = getelementptr inbounds nuw i8, ptr %7, i64 912
  %2267 = getelementptr inbounds nuw i8, ptr %7, i64 928
  %2268 = load i64, ptr %2267, align 16, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2269 = load i64, ptr %2266, align 16, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2270 = sub i64 %2269, %2268
  %2271 = icmp ugt i64 %.lcssa2304, %2270
  br i1 %2271, label %2272, label %2273, !prof !ID

2272:                                             ; preds = %.loopexit2537
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.ID)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %2266, i64 noundef %2268, i64 noundef %.lcssa2304, i64 noundef 8, i64 noundef 80)
          to label %2273 unwind label %2262, !noalias !ID

2273:                                             ; preds = %2272, %.loopexit2537
  %2274 = getelementptr inbounds nuw i8, ptr %7, i64 1016
  %2275 = load i64, ptr %2274, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2276 = icmp ugt i64 %.lcssa2304, %2275
  br i1 %2276, label %2277, label %2163, !prof !ID

2277:                                             ; preds = %2273
  %2278 = getelementptr inbounds nuw i8, ptr %7, i64 1000
; invoke <hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>
  %2279 = invoke { i64, i64 } @<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %2278, i64 noundef %.lcssa2304, ptr noundef nonnull align 8 %2266, i1 noundef zeroext true) #ATTR
          to label %2163 unwind label %2262, !noalias !ID

2280:                                             ; preds = %2981, %2978, %2975
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %74) #ATTR
          to label %2255 unwind label %2152, !noalias !ID

2281:                                             ; preds = %2402, %2177
  %2282 = phi ptr [ %1978, %2177 ], [ %2283, %2402 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2283 = getelementptr inbounds nuw i8, ptr %2282, i64 160
  store ptr %2283, ptr %2178, align 8, !alias.scope !ID, !noalias !ID
  %2284 = load i64, ptr %2282, align 8, !noalias !ID
  %2285 = getelementptr inbounds nuw i8, ptr %2282, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %73, ptr noundef nonnull align 8 dereferenceable(152) %2285, i64 152, i1 false), !noalias !ID
  %2286 = icmp eq i64 %2284, -1
  br i1 %2286, label %.loopexit352, label %2287

2287:                                             ; preds = %2281
  call void @llvm.lifetime.start.p0(ptr nonnull %72), !noalias !ID
  store i64 %2284, ptr %72, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %2180, ptr noundef nonnull align 8 dereferenceable(152) %73, i64 152, i1 false), !noalias !ID
  %2288 = load i64, ptr %2181, align 8, !noalias !ID
  %2289 = load ptr, ptr %2182, align 8, !noalias !ID
  %2290 = load i64, ptr %2183, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %71), !noalias !ID
  %2291 = load ptr, ptr %2180, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %2292 = load i64, ptr %2184, align 8, !noalias !ID, !noundef !ID
  %2293 = icmp ult i64 %2292, 104811045873349726
  call void @llvm.assume(i1 %2293)
  %2294 = getelementptr inbounds nuw [88 x i8], ptr %2291, i64 %2292
  store ptr %2291, ptr %71, align 8, !noalias !ID
  store i64 %2284, ptr %2185, align 8, !noalias !ID
  store ptr %2291, ptr %2186, align 8, !noalias !ID
  store ptr %2294, ptr %2187, align 8, !noalias !ID
  %2295 = load ptr, ptr %2190, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %2296 = load i64, ptr %2189, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2297 = load i64, ptr %2191, align 8, !noalias !ID, !noundef !ID
  %2298 = icmp ult i64 %2297, 288230376151711744
  call void @llvm.assume(i1 %2298)
  %2299 = shl nuw nsw i64 %2297, 5
  %2300 = getelementptr inbounds nuw i8, ptr %2295, i64 %2299
  %2301 = icmp eq i64 %2297, 0
  br i1 %2301, label %.loopexit351, label %2302

2302:                                             ; preds = %2287
  %2303 = load i64, ptr %2188, align 8, !noalias !ID, !noundef !ID
  %2304 = icmp ult i64 %2290, 384307168202282326
  %2305 = ptrtoint ptr %2294 to i64
  br label %2346

.loopexit352:                                     ; preds = %2402, %2281, %2242, %2170
  call void @llvm.lifetime.end.p0(ptr nonnull %73)
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %74)
          to label %2306 unwind label %2262, !noalias !ID

2306:                                             ; preds = %.loopexit352
  call void @llvm.lifetime.end.p0(ptr nonnull %74), !noalias !ID
  %2307 = load i64, ptr %82, align 8, !noalias !ID
  %2308 = load ptr, ptr %2038, align 8, !noalias !ID
  %2309 = load i64, ptr %2039, align 8, !noalias !ID
  %2310 = atomicrmw sub ptr %2155, i64 1 release, align 8, !noalias !ID
  %2311 = icmp eq i64 %2310, 1
  br i1 %2311, label %2312, label %2313

2312:                                             ; preds = %2306
  fence acquire, !noalias !ID
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %75) #ATTR
          to label %2313 unwind label %2084, !noalias !ID

2313:                                             ; preds = %2312, %2306
  call void @llvm.lifetime.end.p0(ptr nonnull %75), !noalias !ID
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %79)
          to label %2314 unwind label %2096, !noalias !ID

2314:                                             ; preds = %2313
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %82), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %91), !noalias !ID
  br label %3039

2315:                                             ; preds = %2584
  %2316 = landingpad { ptr, i32 }
          cleanup
  store ptr %2578, ptr %2186, align 8, !noalias !ID
  br label %2341

2317:                                             ; preds = %2561
  %2318 = landingpad { ptr, i32 }
          cleanup
  br label %2341

2319:                                             ; preds = %2651
  %2320 = landingpad { ptr, i32 }
          cleanup
  store ptr %2645, ptr %2186, align 8, !noalias !ID
  br label %2341

2321:                                             ; preds = %2673
  %2322 = landingpad { ptr, i32 }
          cleanup
  store ptr %2667, ptr %2186, align 8, !noalias !ID
  br label %2341

2323:                                             ; preds = %2873
  %2324 = landingpad { ptr, i32 }
          cleanup
  store ptr %2870, ptr %2045, align 8, !noalias !ID
  br label %2341

2325:                                             ; preds = %2731
  %2326 = landingpad { ptr, i32 }
          cleanup
  store ptr %2725, ptr %2186, align 8, !noalias !ID
  br label %2341

2327:                                             ; preds = %2696, %2680, %2629
  %2328 = landingpad { ptr, i32 }
          cleanup
  br label %2341

2329:                                             ; preds = %2855
  %2330 = landingpad { ptr, i32 }
          cleanup
  store ptr %2849, ptr %2186, align 8, !noalias !ID
  br label %2341

2331:                                             ; preds = %2797
  %2332 = landingpad { ptr, i32 }
          cleanup
  store ptr %2791, ptr %2186, align 8, !noalias !ID
  br label %2341

2333:                                             ; preds = %.preheader347
  %2334 = landingpad { ptr, i32 }
          cleanup
  br label %2341

2335:                                             ; preds = %2423
  %2336 = landingpad { ptr, i32 }
          cleanup
  br label %2341

2337:                                             ; preds = %2825, %2821, %2813, %2805
  %2338 = landingpad { ptr, i32 }
          cleanup
  br label %2341

2339:                                             ; preds = %2437, %2406
  %2340 = landingpad { ptr, i32 }
          cleanup
  br label %2341

2341:                                             ; preds = %2901, %2898, %2339, %2337, %2335, %2333, %2331, %2329, %2327, %2325, %2323, %2321, %2319, %2317, %2315
  %2342 = phi { ptr, i32 } [ %2899, %2898 ], [ %2899, %2901 ], [ %2316, %2315 ], [ %2318, %2317 ], [ %2320, %2319 ], [ %2322, %2321 ], [ %2324, %2323 ], [ %2326, %2325 ], [ %2328, %2327 ], [ %2330, %2329 ], [ %2332, %2331 ], [ %2334, %2333 ], [ %2336, %2335 ], [ %2338, %2337 ], [ %2340, %2339 ]
  %2343 = icmp eq i64 %2296, 0
  br i1 %2343, label %2365, label %2344

2344:                                             ; preds = %2341
  %2345 = shl nuw i64 %2296, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2295, i64 noundef %2345, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %2365

2346:                                             ; preds = %.loopexit329, %2302
  %2347 = phi ptr [ %2291, %2302 ], [ %2712, %.loopexit329 ]
  %2348 = phi i64 [ 0, %2302 ], [ %2435, %.loopexit329 ]
  %2349 = phi i64 [ %2303, %2302 ], [ %2714, %.loopexit329 ]
  %2350 = phi ptr [ %2291, %2302 ], [ %2713, %.loopexit329 ]
  %2351 = phi ptr [ %2295, %2302 ], [ %2352, %.loopexit329 ]
  %2352 = getelementptr inbounds nuw i8, ptr %2351, i64 32
  %2353 = load i64, ptr %2351, align 8, !noalias !ID
  %2354 = getelementptr inbounds nuw i8, ptr %2351, i64 8
  %2355 = load i64, ptr %2354, align 8, !noalias !ID
  %2356 = getelementptr inbounds nuw i8, ptr %2351, i64 16
  %2357 = load i64, ptr %2356, align 8, !noalias !ID
  %2358 = getelementptr inbounds nuw i8, ptr %2351, i64 24
  %2359 = load i64, ptr %2358, align 8, !noalias !ID
  %2360 = icmp eq i64 %2353, 0
  %2361 = select i1 %2360, i1 true, i1 %2158
  br i1 %2361, label %2404, label %2411

.loopexit351:                                     ; preds = %.loopexit329, %2287
  %2362 = icmp eq i64 %2296, 0
  br i1 %2362, label %2366, label %2363

2363:                                             ; preds = %.loopexit351
  %2364 = shl nuw i64 %2296, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2295, i64 noundef %2364, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %2366

2365:                                             ; preds = %2344, %2341
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %71) #ATTR
          to label %2367 unwind label %2152, !noalias !ID

2366:                                             ; preds = %2363, %.loopexit351
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %71)
          to label %2377 unwind label %2373, !noalias !ID

2367:                                             ; preds = %2375, %2373, %2365
  %2368 = phi i8 [ 1, %2365 ], [ 1, %2373 ], [ %2922, %2375 ]
  %2369 = phi { ptr, i32 } [ %2342, %2365 ], [ %2374, %2373 ], [ %2376, %2375 ]
  %2370 = icmp eq i64 %2288, 0
  br i1 %2370, label %2381, label %2371

2371:                                             ; preds = %2367
  %2372 = mul nuw i64 %2288, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2289) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2289, i64 noundef %2372, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %2381

2373:                                             ; preds = %2366
  %2374 = landingpad { ptr, i32 }
          cleanup
  br label %2367

2375:                                             ; preds = %2927
  %2376 = landingpad { ptr, i32 }
          cleanup
  br label %2367

2377:                                             ; preds = %2366
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !ID
  %2378 = icmp eq i64 %2288, 0
  br i1 %2378, label %2388, label %2379

2379:                                             ; preds = %2377
  %2380 = mul nuw i64 %2288, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2289) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2289, i64 noundef %2380, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %2388

2381:                                             ; preds = %2371, %2367
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2382 = load ptr, ptr %2228, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2383 = icmp eq ptr %2382, null
  br i1 %2383, label %2975, label %2384

2384:                                             ; preds = %2381
  %2385 = atomicrmw sub ptr %2382, i64 1 release, align 8, !noalias !ID
  %2386 = icmp eq i64 %2385, 1
  br i1 %2386, label %2387, label %2975

2387:                                             ; preds = %2384
  fence acquire, !noalias !ID
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2228) #ATTR, !noalias !ID
  br label %2975

2388:                                             ; preds = %2379, %2377
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2389 = load ptr, ptr %2228, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2390 = icmp eq ptr %2389, null
  br i1 %2390, label %2395, label %2391

2391:                                             ; preds = %2388
  %2392 = atomicrmw sub ptr %2389, i64 1 release, align 8, !noalias !ID
  %2393 = icmp eq i64 %2392, 1
  br i1 %2393, label %2394, label %2395

2394:                                             ; preds = %2391
  fence acquire, !noalias !ID
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2228) #ATTR, !noalias !ID
  br label %2395

2395:                                             ; preds = %2394, %2391, %2388
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2396 = load ptr, ptr %2229, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2397 = icmp eq ptr %2396, null
  br i1 %2397, label %2402, label %2398

2398:                                             ; preds = %2395
  %2399 = atomicrmw sub ptr %2396, i64 1 release, align 8, !noalias !ID
  %2400 = icmp eq i64 %2399, 1
  br i1 %2400, label %2401, label %2402

2401:                                             ; preds = %2398
  fence acquire, !noalias !ID
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2229) #ATTR, !noalias !ID
  br label %2402

2402:                                             ; preds = %2401, %2398, %2395
  call void @llvm.lifetime.end.p0(ptr nonnull %72), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %73)
  call void @llvm.lifetime.start.p0(ptr nonnull %73)
  %2403 = icmp eq ptr %2283, %2179
  br i1 %2403, label %.loopexit352, label %2281

2404:                                             ; preds = %2427, %2346
  call void @llvm.assume(i1 %2304)
  %2405 = icmp ugt i64 %2348, %2290
  br i1 %2405, label %2406, label %2432, !prof !ID

2406:                                             ; preds = %2404
  call void @llvm.lifetime.start.p0(ptr nonnull %47), !noalias !ID
  store i64 %2348, ptr %47, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %46), !noalias !ID
  store i64 %2290, ptr %46, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !ID
  store ptr %47, ptr %45, align 8, !noalias !ID
  %2407 = getelementptr inbounds nuw i8, ptr %45, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %2407, align 8, !noalias !ID
  %2408 = getelementptr inbounds nuw i8, ptr %45, i64 16
  store ptr %46, ptr %2408, align 8, !noalias !ID
  %2409 = getelementptr inbounds nuw i8, ptr %45, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %2409, align 8, !noalias !ID
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.HASH.2158, ptr noundef nonnull %45, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.289) #ATTR
          to label %2410 unwind label %2339, !noalias !ID

2410:                                             ; preds = %2406
  unreachable

2411:                                             ; preds = %2346
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !ID
  %2412 = load atomic i64, ptr %2192 monotonic, align 8, !noalias !ID
  br label %2413

2413:                                             ; preds = %2413, %2411
  %2414 = phi i64 [ %2412, %2411 ], [ %2418, %2413 ]
  %2415 = call i64 @llvm.uadd.sat.i64(i64 %2414, i64 %2353)
  %2416 = cmpxchg weak ptr %2192, i64 %2414, i64 %2415 monotonic monotonic, align 8, !noalias !ID
  %2417 = extractvalue { i64, i1 } %2416, 1
  %2418 = extractvalue { i64, i1 } %2416, 0
  br i1 %2417, label %2419, label %2413

2419:                                             ; preds = %2413
  %2420 = call i64 @llvm.uadd.sat.i64(i64 %2418, i64 %2353)
  %2421 = load i64, ptr %2156, align 8, !noalias !ID
  %2422 = icmp ugt i64 %2420, %2421
  br i1 %2422, label %2423, label %2427

2423:                                             ; preds = %2419
  call void @llvm.lifetime.start.p0(ptr nonnull %43), !noalias !ID
  store i8 0, ptr %2193, align 1, !noalias !ID
  store i64 %2421, ptr %2194, align 8, !noalias !ID
  store i64 %2420, ptr %2195, align 8, !noalias !ID
  store i8 0, ptr %43, align 8, !noalias !ID
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.ID)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %44, ptr noundef nonnull align 8 %2156, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %43)
          to label %2424 unwind label %2335, !noalias !ID

2424:                                             ; preds = %2423
  call void @llvm.lifetime.end.p0(ptr nonnull %43), !noalias !ID
  %2425 = load i8, ptr %44, align 8, !noalias !ID
  %2426 = icmp eq i8 %2425, -1
  br i1 %2426, label %2427, label %2428

2427:                                             ; preds = %2424, %2419
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !ID
  br label %2404

2428:                                             ; preds = %2424
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !ID
  %2429 = load i64, ptr %82, align 8, !noalias !ID
  %2430 = load ptr, ptr %2038, align 8, !noalias !ID
  %2431 = load i64, ptr %2039, align 8, !noalias !ID
  br label %2914

2432:                                             ; preds = %2404
  %2433 = icmp ult i64 %2355, %2348
  %2434 = call i64 @llvm.umin.i64(i64 %2355, i64 range(i64 0, 384307168202282326) %2290)
  %2435 = select i1 %2433, i64 %2348, i64 %2434
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2289) ]
  %2436 = icmp samesign ult i64 %2435, %2348
  br i1 %2436, label %2437, label %2438, !prof !ID

2437:                                             ; preds = %2432
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %2348, i64 noundef %2435, i64 noundef %2290, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.290) #ATTR
          to label %2926 unwind label %2339, !noalias !ID

2438:                                             ; preds = %2432
  %2439 = mul nuw nsw i64 %2348, 24
  %2440 = getelementptr inbounds nuw i8, ptr %2289, i64 %2439
  %2441 = mul nuw nsw i64 %2435, 24
  %2442 = getelementptr inbounds nuw i8, ptr %2289, i64 %2441
  %2443 = icmp eq i64 %2348, %2435
  br i1 %2443, label %.loopexit350, label %2444

2444:                                             ; preds = %2438
  %2445 = sub nuw nsw i64 %2441, %2439
  %2446 = udiv exact i64 %2445, 24
  br label %2447

2447:                                             ; preds = %2462, %2444
  %2448 = phi i64 [ 0, %2444 ], [ %2463, %2462 ]
  %2449 = phi i64 [ 0, %2444 ], [ %2464, %2462 ]
  %2450 = phi i64 [ 0, %2444 ], [ %2465, %2462 ]
  %2451 = phi i64 [ 0, %2444 ], [ %2466, %2462 ]
  %2452 = getelementptr inbounds nuw [24 x i8], ptr %2440, i64 %2451
  %2453 = load i8, ptr %2452, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2454 = getelementptr i8, ptr %2452, i64 8
  %2455 = load i64, ptr %2454, align 8, !noalias !ID
  switch i8 %2453, label %.unreachabledefault [
    i8 0, label %2456
    i8 1, label %2458
    i8 2, label %2462
    i8 3, label %2460
  ]

.unreachabledefault:                              ; preds = %2447
  unreachable

default.unreachable1591:                          ; preds = %.preheader333
  unreachable

2456:                                             ; preds = %2447
  %2457 = call i64 @llvm.uadd.sat.i64(i64 %2450, i64 %2455)
  br label %2462

2458:                                             ; preds = %2447
  %2459 = call i64 @llvm.uadd.sat.i64(i64 %2449, i64 %2455)
  br label %2462

2460:                                             ; preds = %2447
  %2461 = call i64 @llvm.umax.i64(i64 %2448, i64 %2455)
  br label %2462

2462:                                             ; preds = %2460, %2458, %2456, %2447
  %2463 = phi i64 [ %2448, %2456 ], [ %2448, %2458 ], [ %2461, %2460 ], [ %2448, %2447 ]
  %2464 = phi i64 [ %2449, %2456 ], [ %2459, %2458 ], [ %2449, %2460 ], [ %2449, %2447 ]
  %2465 = phi i64 [ %2457, %2456 ], [ %2450, %2458 ], [ %2450, %2460 ], [ %2450, %2447 ]
  %2466 = add nuw i64 %2451, 1
  %2467 = icmp eq i64 %2466, %2446
  br i1 %2467, label %.loopexit350, label %2447

.loopexit350:                                     ; preds = %2462, %2438
  %2468 = phi i64 [ 0, %2438 ], [ %2465, %2462 ]
  %2469 = phi i64 [ 0, %2438 ], [ %2464, %2462 ]
  %2470 = phi i64 [ 0, %2438 ], [ %2463, %2462 ]
  br i1 %2161, label %2474, label %.loopexit348

.loopexit348:                                     ; preds = %2486, %2474, %.loopexit350
  %2471 = phi i64 [ 0, %.loopexit350 ], [ 0, %2474 ], [ %2488, %2486 ]
  %2472 = load atomic i32, ptr %2196 acquire, align 8, !noalias !ID
  %2473 = icmp eq i32 %2472, 0
  br i1 %2473, label %2490, label %2493

2474:                                             ; preds = %.loopexit350
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2350) ]
  %2475 = ptrtoint ptr %2350 to i64
  %2476 = call i64 @llvm.usub.sat.i64(i64 %2357, i64 %2349)
  %2477 = sub nuw i64 %2305, %2475
  %2478 = udiv exact i64 %2477, 88
  %2479 = call i64 @llvm.umin.i64(i64 %2476, i64 %2478)
  %2480 = icmp eq i64 %2479, 0
  br i1 %2480, label %.loopexit348, label %.preheader347

.preheader347:                                    ; preds = %2474, %2486
  %2481 = phi i64 [ %2488, %2486 ], [ 0, %2474 ]
  %2482 = phi i64 [ %2487, %2486 ], [ 0, %2474 ]
  %2483 = getelementptr inbounds nuw [88 x i8], ptr %2350, i64 %2482
  %2484 = getelementptr inbounds nuw i8, ptr %2483, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %2485 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %2484)
          to label %2486 unwind label %2333, !noalias !ID

2486:                                             ; preds = %.preheader347
  %2487 = add nuw nsw i64 %2482, 1
  %2488 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %2481, i64 %2485)
  %2489 = icmp eq i64 %2487, %2479
  br i1 %2489, label %.loopexit348, label %.preheader347

2490:                                             ; preds = %.loopexit348
  %2491 = load i8, ptr %2197, align 8, !noalias !ID
  %2492 = icmp eq i8 %2491, -1
  br i1 %2492, label %2493, label %2500

2493:                                             ; preds = %2490, %.loopexit348
  br i1 %2158, label %2494, label %2495

2494:                                             ; preds = %2495, %2493
  br i1 %2161, label %2503, label %2501

2495:                                             ; preds = %2493
  %2496 = load atomic i64, ptr %2192 monotonic, align 8, !noalias !ID
  %2497 = load i64, ptr %2156, align 8, !noalias !ID
  %2498 = call i64 @llvm.uadd.sat.i64(i64 %2496, i64 %2468)
  %2499 = icmp ugt i64 %2498, %2497
  br i1 %2499, label %2500, label %2494

2500:                                             ; preds = %2503, %2495, %2490
  br i1 %2443, label %.loopexit335, label %.preheader333

2501:                                             ; preds = %2503, %2494
  %2502 = or i1 %2158, %2443
  br i1 %2502, label %.loopexit346, label %.preheader345

2503:                                             ; preds = %2494
  %2504 = load i64, ptr %2198, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2505 = load atomic i64, ptr %2199 monotonic, align 16, !alias.scope !ID, !noalias !ID
  %2506 = call noundef i64 @llvm.usub.sat.i64(i64 %2504, i64 %2505)
  %2507 = call i64 @llvm.uadd.sat.i64(i64 %2506, i64 %2471)
  %2508 = call i64 @llvm.uadd.sat.i64(i64 %2507, i64 %2469)
  %2509 = call i64 @llvm.uadd.sat.i64(i64 %2508, i64 %2470)
  %2510 = load atomic i64, ptr %2200 monotonic, align 8, !noalias !ID
  %2511 = load i64, ptr %2159, align 8, !noalias !ID
  %2512 = call i64 @llvm.uadd.sat.i64(i64 %2510, i64 %2509)
  %2513 = icmp ugt i64 %2512, %2511
  br i1 %2513, label %2500, label %2501

.preheader333:                                    ; preds = %2500, %2698
  %2514 = phi ptr [ %2699, %2698 ], [ %2347, %2500 ]
  %2515 = phi ptr [ %2519, %2698 ], [ %2440, %2500 ]
  %2516 = phi i64 [ %2702, %2698 ], [ %2349, %2500 ]
  %2517 = phi ptr [ %2701, %2698 ], [ %2350, %2500 ]
  %2518 = phi ptr [ %2700, %2698 ], [ %2347, %2500 ]
  %2519 = getelementptr inbounds nuw i8, ptr %2515, i64 24
  %2520 = load i8, ptr %2515, align 8, !range !ID, !noalias !ID, !noundef !ID
  switch i8 %2520, label %default.unreachable1591 [
    i8 0, label %2530
    i8 1, label %2537
    i8 2, label %2542
    i8 3, label %2548
  ]

.loopexit335:                                     ; preds = %2698, %2500
  %2521 = phi ptr [ %2347, %2500 ], [ %2699, %2698 ]
  %2522 = phi ptr [ %2350, %2500 ], [ %2701, %2698 ]
  %2523 = phi i64 [ %2349, %2500 ], [ %2702, %2698 ]
  %2524 = icmp ult i64 %2523, %2357
  %2525 = select i1 %2161, i1 %2524, i1 false
  br i1 %2525, label %2526, label %2711

2526:                                             ; preds = %.loopexit335
  call void @llvm.lifetime.start.p0(ptr nonnull %49)
  %2527 = icmp eq ptr %2522, %2294
  br i1 %2527, label %.loopexit331, label %2528

2528:                                             ; preds = %2526
  %2529 = add i64 %2357, -1
  br label %2722

2530:                                             ; preds = %.preheader333
  %2531 = getelementptr inbounds nuw i8, ptr %2515, i64 1
  %2532 = load i8, ptr %2531, align 1, !range !ID, !noalias !ID, !noundef !ID
  %2533 = getelementptr inbounds nuw i8, ptr %2515, i64 8
  %2534 = load i64, ptr %2533, align 8, !noalias !ID, !noundef !ID
  %2535 = getelementptr inbounds nuw i8, ptr %2515, i64 16
  %2536 = load i64, ptr %2535, align 8, !noalias !ID, !noundef !ID
  br i1 %2158, label %2698, label %2549

2537:                                             ; preds = %.preheader333
  %2538 = getelementptr inbounds nuw i8, ptr %2515, i64 8
  %2539 = load i64, ptr %2538, align 8, !noalias !ID, !noundef !ID
  %2540 = getelementptr inbounds nuw i8, ptr %2515, i64 16
  %2541 = load i64, ptr %2540, align 8, !noalias !ID, !noundef !ID
  br i1 %2161, label %2608, label %2698

2542:                                             ; preds = %.preheader333
  %2543 = getelementptr inbounds nuw i8, ptr %2515, i64 8
  %2544 = load i64, ptr %2543, align 8, !noalias !ID, !noundef !ID
  %2545 = icmp ult i64 %2516, %2544
  br i1 %2545, label %2546, label %.loopexit328

2546:                                             ; preds = %2542
  call void @llvm.lifetime.start.p0(ptr nonnull %55)
  %2547 = icmp eq ptr %2517, %2294
  br i1 %2547, label %.loopexit327, label %.preheader326

2548:                                             ; preds = %.preheader333
  br i1 %2161, label %2684, label %2698

2549:                                             ; preds = %2530
  call void @llvm.lifetime.start.p0(ptr nonnull %42), !noalias !ID
  %2550 = load atomic i64, ptr %2192 monotonic, align 8, !noalias !ID
  br label %2551

2551:                                             ; preds = %2551, %2549
  %2552 = phi i64 [ %2550, %2549 ], [ %2556, %2551 ]
  %2553 = call i64 @llvm.uadd.sat.i64(i64 %2552, i64 %2534)
  %2554 = cmpxchg weak ptr %2192, i64 %2552, i64 %2553 monotonic monotonic, align 8, !noalias !ID
  %2555 = extractvalue { i64, i1 } %2554, 1
  %2556 = extractvalue { i64, i1 } %2554, 0
  br i1 %2555, label %2557, label %2551

2557:                                             ; preds = %2551
  %2558 = call i64 @llvm.uadd.sat.i64(i64 %2556, i64 %2534)
  %2559 = load i64, ptr %2156, align 8, !noalias !ID
  %2560 = icmp ugt i64 %2558, %2559
  br i1 %2560, label %2561, label %2567

2561:                                             ; preds = %2557
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !ID
  store i8 0, ptr %2215, align 1, !noalias !ID
  store i64 %2559, ptr %2216, align 8, !noalias !ID
  store i64 %2558, ptr %2217, align 8, !noalias !ID
  store i8 0, ptr %41, align 8, !noalias !ID
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.ID)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %42, ptr noundef nonnull align 8 %2156, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %41)
          to label %2562 unwind label %2317, !noalias !ID

2562:                                             ; preds = %2561
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !ID
  %2563 = load i8, ptr %42, align 8, !noalias !ID
  %2564 = icmp eq i8 %2563, -1
  br i1 %2564, label %2567, label %2565

2565:                                             ; preds = %2562
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !ID
  %2566 = icmp eq i64 %2536, 0
  br i1 %2566, label %.loopexit334, label %2569

2567:                                             ; preds = %2562, %2557
  call void @llvm.lifetime.end.p0(ptr nonnull %42), !noalias !ID
  %2568 = icmp eq i8 %2532, -1
  br i1 %2568, label %2698, label %2589

2569:                                             ; preds = %2565
  %2570 = add i64 %2536, -1
  %2571 = icmp ult i64 %2516, %2570
  br i1 %2571, label %2572, label %.loopexit322

2572:                                             ; preds = %2569
  call void @llvm.lifetime.start.p0(ptr nonnull %53)
  %2573 = icmp eq ptr %2517, %2294
  br i1 %2573, label %.loopexit321, label %.preheader320

2574:                                             ; preds = %2586
  call void @llvm.lifetime.start.p0(ptr nonnull %53)
  %2575 = icmp eq ptr %2578, %2294
  br i1 %2575, label %.loopexit321, label %.preheader320

.preheader320:                                    ; preds = %2572, %2574
  %2576 = phi ptr [ %2578, %2574 ], [ %2517, %2572 ]
  %2577 = phi i64 [ %2587, %2574 ], [ %2516, %2572 ]
  %2578 = getelementptr inbounds nuw i8, ptr %2576, i64 88
  %2579 = load i64, ptr %2576, align 8, !noalias !ID
  %2580 = getelementptr inbounds nuw i8, ptr %2576, i64 8
  %2581 = load i64, ptr %2580, align 8, !noalias !ID
  %2582 = getelementptr inbounds nuw i8, ptr %2576, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %53, ptr noundef nonnull align 8 dereferenceable(72) %2582, i64 72, i1 false), !noalias !ID
  %2583 = icmp eq i64 %2581, -1
  br i1 %2583, label %.loopexit321, label %2584

2584:                                             ; preds = %.preheader320
  call void @llvm.lifetime.start.p0(ptr nonnull %54), !noalias !ID
  store i64 %2581, ptr %54, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %2218, ptr noundef nonnull align 8 dereferenceable(72) %53, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1299, i64 noundef %2579, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %54)
          to label %2586 unwind label %2315, !noalias !ID

.loopexit321:                                     ; preds = %.preheader320, %2574, %2572
  %2585 = phi ptr [ %2518, %2572 ], [ %2578, %2574 ], [ %2578, %.preheader320 ]
  store ptr %2585, ptr %2186, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  br label %.loopexit334

2586:                                             ; preds = %2584
  %2587 = add i64 %2577, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %54), !noalias !ID
  %2588 = icmp eq i64 %2587, %2570
  br i1 %2588, label %.loopexit322, label %2574

2589:                                             ; preds = %2567
  %2590 = load ptr, ptr %2201, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2591 = icmp eq ptr %2590, null
  br i1 %2591, label %2698, label %2592

2592:                                             ; preds = %2589
  %2593 = load i32, ptr %2202, align 4, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2594 = getelementptr i8, ptr %2590, i64 56
  %2595 = load i64, ptr %2594, align 8, !noalias !ID, !noundef !ID
  %2596 = zext i32 %2593 to i64
  %2597 = icmp ugt i64 %2595, %2596
  br i1 %2597, label %2598, label %2698

2598:                                             ; preds = %2592
  %2599 = getelementptr i8, ptr %2590, i64 48
  %2600 = load ptr, ptr %2599, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %2601 = zext nneg i8 %2532 to i64
  %2602 = getelementptr inbounds nuw [136 x i8], ptr %2600, i64 %2596
  %2603 = getelementptr inbounds nuw [8 x i8], ptr %2602, i64 %2601
  %2604 = atomicrmw add ptr %2603, i64 %2534 monotonic, align 8, !noalias !ID
  br label %2698

.loopexit322:                                     ; preds = %2586, %2569
  %2605 = phi ptr [ %2518, %2569 ], [ %2578, %2586 ]
  store ptr %2605, ptr %2186, align 8, !noalias !ID
  br label %.loopexit334

.loopexit325:                                     ; preds = %2653, %2636
  %2606 = phi ptr [ %2518, %2636 ], [ %2645, %2653 ]
  store ptr %2606, ptr %2186, align 8, !noalias !ID
  br label %.loopexit334

2607:                                             ; preds = %2631
  br i1 %2633, label %.loopexit334, label %2698

2608:                                             ; preds = %2537
  call void @llvm.lifetime.start.p0(ptr nonnull %67), !noalias !ID
  %2609 = load i64, ptr %2159, align 8, !noalias !ID
  %2610 = icmp eq i64 %2609, -1
  br i1 %2610, label %2611, label %2612

2611:                                             ; preds = %2608
  call void @llvm.lifetime.end.p0(ptr nonnull %67), !noalias !ID
  br label %2698

2612:                                             ; preds = %2608
  %2613 = load atomic i32, ptr %2196 acquire, align 8, !noalias !ID
  %2614 = icmp eq i32 %2613, 0
  br i1 %2614, label %2627, label %2615

2615:                                             ; preds = %2612
  %2616 = load atomic i64, ptr %2200 monotonic, align 8, !noalias !ID
  br label %2617

2617:                                             ; preds = %2617, %2615
  %2618 = phi i64 [ %2616, %2615 ], [ %2622, %2617 ]
  %2619 = call i64 @llvm.uadd.sat.i64(i64 %2618, i64 %2539)
  %2620 = cmpxchg weak ptr %2200, i64 %2618, i64 %2619 monotonic monotonic, align 8, !noalias !ID
  %2621 = extractvalue { i64, i1 } %2620, 1
  %2622 = extractvalue { i64, i1 } %2620, 0
  br i1 %2621, label %2623, label %2617

2623:                                             ; preds = %2617
  %2624 = call i64 @llvm.uadd.sat.i64(i64 %2622, i64 %2539)
  %2625 = load i64, ptr %2159, align 8, !noalias !ID
  %2626 = icmp ugt i64 %2624, %2625
  br i1 %2626, label %2629, label %2628

2627:                                             ; preds = %2612
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %67, ptr noundef nonnull align 8 dereferenceable(24) %2197, i64 24, i1 false), !noalias !ID
  br label %2631

2628:                                             ; preds = %2623
  call void @llvm.lifetime.end.p0(ptr nonnull %67), !noalias !ID
  br label %2698

2629:                                             ; preds = %2623
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !ID
  store i8 3, ptr %2211, align 1, !noalias !ID
  store i64 %2625, ptr %2212, align 8, !noalias !ID
  store i64 %2624, ptr %2213, align 8, !noalias !ID
  store i8 0, ptr %37, align 8, !noalias !ID
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.ID)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %67, ptr noundef nonnull align 8 %2156, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %37)
          to label %2630 unwind label %2327, !noalias !ID

2630:                                             ; preds = %2629
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !ID
  br label %2631

2631:                                             ; preds = %2630, %2627
  %2632 = load i8, ptr %67, align 8, !noalias !ID
  %2633 = icmp ne i8 %2632, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %67), !noalias !ID
  %2634 = icmp ne i64 %2541, 0
  %2635 = and i1 %2634, %2633
  br i1 %2635, label %2636, label %2607

2636:                                             ; preds = %2631
  %2637 = add i64 %2541, -1
  %2638 = icmp ult i64 %2516, %2637
  br i1 %2638, label %2639, label %.loopexit325

2639:                                             ; preds = %2636
  call void @llvm.lifetime.start.p0(ptr nonnull %51)
  %2640 = icmp eq ptr %2517, %2294
  br i1 %2640, label %.loopexit324, label %.preheader323

2641:                                             ; preds = %2653
  call void @llvm.lifetime.start.p0(ptr nonnull %51)
  %2642 = icmp eq ptr %2645, %2294
  br i1 %2642, label %.loopexit324, label %.preheader323

.preheader323:                                    ; preds = %2639, %2641
  %2643 = phi ptr [ %2645, %2641 ], [ %2517, %2639 ]
  %2644 = phi i64 [ %2654, %2641 ], [ %2516, %2639 ]
  %2645 = getelementptr inbounds nuw i8, ptr %2643, i64 88
  %2646 = load i64, ptr %2643, align 8, !noalias !ID
  %2647 = getelementptr inbounds nuw i8, ptr %2643, i64 8
  %2648 = load i64, ptr %2647, align 8, !noalias !ID
  %2649 = getelementptr inbounds nuw i8, ptr %2643, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %51, ptr noundef nonnull align 8 dereferenceable(72) %2649, i64 72, i1 false), !noalias !ID
  %2650 = icmp eq i64 %2648, -1
  br i1 %2650, label %.loopexit324, label %2651

2651:                                             ; preds = %.preheader323
  call void @llvm.lifetime.start.p0(ptr nonnull %52), !noalias !ID
  store i64 %2648, ptr %52, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %2214, ptr noundef nonnull align 8 dereferenceable(72) %51, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %51)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1299, i64 noundef %2646, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %52)
          to label %2653 unwind label %2319, !noalias !ID

.loopexit324:                                     ; preds = %.preheader323, %2641, %2639
  %2652 = phi ptr [ %2518, %2639 ], [ %2645, %2641 ], [ %2645, %.preheader323 ]
  store ptr %2652, ptr %2186, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %51)
  br label %.loopexit334

2653:                                             ; preds = %2651
  %2654 = add i64 %2644, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %52), !noalias !ID
  %2655 = icmp eq i64 %2654, %2637
  br i1 %2655, label %.loopexit325, label %2641

2656:                                             ; preds = %2677
  call void @llvm.lifetime.start.p0(ptr nonnull %55)
  %2657 = icmp eq ptr %2667, %2294
  br i1 %2657, label %.loopexit327, label %.preheader326

.loopexit328:                                     ; preds = %2677, %2542
  %2658 = phi ptr [ %2518, %2542 ], [ %2667, %2677 ]
  %2659 = phi ptr [ %2517, %2542 ], [ %2667, %2677 ]
  %2660 = phi i64 [ %2516, %2542 ], [ %2544, %2677 ]
  store ptr %2658, ptr %2186, align 8, !noalias !ID
  br label %2661

2661:                                             ; preds = %.loopexit327, %.loopexit328
  %2662 = phi ptr [ %2675, %.loopexit327 ], [ %2658, %.loopexit328 ]
  %2663 = phi i64 [ %2674, %.loopexit327 ], [ %2660, %.loopexit328 ]
  %2664 = phi ptr [ %2676, %.loopexit327 ], [ %2659, %.loopexit328 ]
  br i1 %2161, label %2680, label %2698

.preheader326:                                    ; preds = %2546, %2656
  %2665 = phi ptr [ %2667, %2656 ], [ %2517, %2546 ]
  %2666 = phi i64 [ %2678, %2656 ], [ %2516, %2546 ]
  %2667 = getelementptr inbounds nuw i8, ptr %2665, i64 88
  %2668 = load i64, ptr %2665, align 8, !noalias !ID
  %2669 = getelementptr inbounds nuw i8, ptr %2665, i64 8
  %2670 = load i64, ptr %2669, align 8, !noalias !ID
  %2671 = getelementptr inbounds nuw i8, ptr %2665, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %55, ptr noundef nonnull align 8 dereferenceable(72) %2671, i64 72, i1 false), !noalias !ID
  %2672 = icmp eq i64 %2670, -1
  br i1 %2672, label %.loopexit327, label %2673

2673:                                             ; preds = %.preheader326
  call void @llvm.lifetime.start.p0(ptr nonnull %56), !noalias !ID
  store i64 %2670, ptr %56, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %2210, ptr noundef nonnull align 8 dereferenceable(72) %55, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1299, i64 noundef %2668, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %56)
          to label %2677 unwind label %2321, !noalias !ID

.loopexit327:                                     ; preds = %.preheader326, %2656, %2546
  %2674 = phi i64 [ %2516, %2546 ], [ %2678, %2656 ], [ %2666, %.preheader326 ]
  %2675 = phi ptr [ %2518, %2546 ], [ %2667, %2656 ], [ %2667, %.preheader326 ]
  %2676 = phi ptr [ %2517, %2546 ], [ %2667, %2656 ], [ %2667, %.preheader326 ]
  store ptr %2675, ptr %2186, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
  br label %2661

2677:                                             ; preds = %2673
  %2678 = add i64 %2666, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %56), !noalias !ID
  %2679 = icmp eq i64 %2678, %2544
  br i1 %2679, label %.loopexit328, label %2656

2680:                                             ; preds = %2661
  call void @llvm.lifetime.start.p0(ptr nonnull %66), !noalias !ID
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %66, ptr noundef nonnull align 16 dereferenceable(1248) %7)
          to label %2681 unwind label %2327, !noalias !ID

2681:                                             ; preds = %2680
  %2682 = load i8, ptr %66, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2683 = icmp eq i8 %2682, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %66), !noalias !ID
  br i1 %2683, label %2698, label %.loopexit334

2684:                                             ; preds = %2548
  %2685 = getelementptr inbounds nuw i8, ptr %2515, i64 8
  %2686 = load i64, ptr %2685, align 8, !noalias !ID, !noundef !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %65), !noalias !ID
  %2687 = load atomic i32, ptr %2196 acquire, align 8, !noalias !ID
  %2688 = icmp eq i32 %2687, 0
  br i1 %2688, label %2689, label %2690

2689:                                             ; preds = %2684
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %65, ptr noundef nonnull align 8 dereferenceable(24) %2197, i64 24, i1 false), !noalias !ID
  br label %2704

2690:                                             ; preds = %2684
  %2691 = load atomic i64, ptr %2200 monotonic, align 8, !noalias !ID
  %2692 = call i64 @llvm.uadd.sat.i64(i64 %2691, i64 %2686)
  %2693 = load i64, ptr %2159, align 8, !noalias !ID
  %2694 = icmp ugt i64 %2692, %2693
  br i1 %2694, label %2696, label %2695

2695:                                             ; preds = %2690
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !ID
  br label %2698

2696:                                             ; preds = %2690
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !ID
  store i8 3, ptr %2207, align 1, !noalias !ID
  store i64 %2693, ptr %2208, align 8, !noalias !ID
  store i64 %2692, ptr %2209, align 8, !noalias !ID
  store i8 0, ptr %40, align 8, !noalias !ID
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.ID)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %65, ptr noundef nonnull align 8 %2156, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %40)
          to label %2697 unwind label %2327, !noalias !ID

2697:                                             ; preds = %2696
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !ID
  br label %2704

2698:                                             ; preds = %2704, %2695, %2681, %2661, %2628, %2611, %2607, %2598, %2592, %2589, %2567, %2548, %2537, %2530
  %2699 = phi ptr [ %2514, %2607 ], [ %2662, %2661 ], [ %2514, %2611 ], [ %2514, %2537 ], [ %2514, %2548 ], [ %2514, %2695 ], [ %2514, %2704 ], [ %2514, %2628 ], [ %2662, %2681 ], [ %2514, %2598 ], [ %2514, %2530 ], [ %2514, %2592 ], [ %2514, %2589 ], [ %2514, %2567 ]
  %2700 = phi ptr [ %2518, %2607 ], [ %2662, %2661 ], [ %2518, %2611 ], [ %2518, %2537 ], [ %2518, %2548 ], [ %2518, %2695 ], [ %2518, %2704 ], [ %2518, %2628 ], [ %2662, %2681 ], [ %2518, %2598 ], [ %2518, %2530 ], [ %2518, %2592 ], [ %2518, %2589 ], [ %2518, %2567 ]
  %2701 = phi ptr [ %2517, %2607 ], [ %2664, %2661 ], [ %2517, %2611 ], [ %2517, %2537 ], [ %2517, %2548 ], [ %2517, %2695 ], [ %2517, %2704 ], [ %2517, %2628 ], [ %2664, %2681 ], [ %2517, %2598 ], [ %2517, %2530 ], [ %2517, %2592 ], [ %2517, %2589 ], [ %2517, %2567 ]
  %2702 = phi i64 [ %2516, %2607 ], [ %2663, %2661 ], [ %2516, %2611 ], [ %2516, %2537 ], [ %2516, %2548 ], [ %2516, %2695 ], [ %2516, %2704 ], [ %2516, %2628 ], [ %2663, %2681 ], [ %2516, %2598 ], [ %2516, %2530 ], [ %2516, %2592 ], [ %2516, %2589 ], [ %2516, %2567 ]
  %2703 = icmp eq ptr %2519, %2442
  br i1 %2703, label %.loopexit335, label %.preheader333

2704:                                             ; preds = %2697, %2689
  %2705 = load i8, ptr %65, align 8, !noalias !ID
  %2706 = icmp eq i8 %2705, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %65), !noalias !ID
  br i1 %2706, label %2698, label %.loopexit334

.loopexit334:                                     ; preds = %2704, %2681, %2607, %.loopexit324, %.loopexit325, %.loopexit322, %.loopexit321, %2565
  %2707 = load i64, ptr %82, align 8, !noalias !ID
  %2708 = load ptr, ptr %2038, align 8, !noalias !ID
  %2709 = load i64, ptr %2039, align 8, !noalias !ID
  br label %2914

2710:                                             ; preds = %2735
  store ptr %2725, ptr %2186, align 8, !noalias !ID
  br label %2711

2711:                                             ; preds = %2843, %2804, %.loopexit331, %2710, %.loopexit335
  %2712 = phi ptr [ %2521, %.loopexit335 ], [ %2782, %2804 ], [ %2732, %.loopexit331 ], [ %2844, %2843 ], [ %2725, %2710 ]
  %2713 = phi ptr [ %2522, %.loopexit335 ], [ %2783, %2804 ], [ %2734, %.loopexit331 ], [ %2846, %2843 ], [ %2725, %2710 ]
  %2714 = phi i64 [ %2523, %.loopexit335 ], [ %2784, %2804 ], [ %2733, %.loopexit331 ], [ %2845, %2843 ], [ %2357, %2710 ]
  %2715 = load ptr, ptr %2045, align 8, !noalias !ID
  %2716 = icmp eq i64 %2359, 0
  br i1 %2716, label %.loopexit329, label %2717

2717:                                             ; preds = %2711
  %2718 = load ptr, ptr %2046, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  br label %2862

2719:                                             ; preds = %2735
  %2720 = add i64 %2723, 1
  call void @llvm.lifetime.start.p0(ptr nonnull %49)
  %2721 = icmp eq ptr %2725, %2294
  br i1 %2721, label %.loopexit331, label %2722

2722:                                             ; preds = %2719, %2528
  %2723 = phi i64 [ %2720, %2719 ], [ %2523, %2528 ]
  %2724 = phi ptr [ %2725, %2719 ], [ %2522, %2528 ]
  %2725 = getelementptr inbounds nuw i8, ptr %2724, i64 88
  %2726 = load i64, ptr %2724, align 8, !noalias !ID
  %2727 = getelementptr inbounds nuw i8, ptr %2724, i64 8
  %2728 = load i64, ptr %2727, align 8, !noalias !ID
  %2729 = getelementptr inbounds nuw i8, ptr %2724, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %49, ptr noundef nonnull align 8 dereferenceable(72) %2729, i64 72, i1 false), !noalias !ID
  %2730 = icmp eq i64 %2728, -1
  br i1 %2730, label %.loopexit331, label %2731

2731:                                             ; preds = %2722
  call void @llvm.lifetime.start.p0(ptr nonnull %50), !noalias !ID
  store i64 %2728, ptr %50, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %2219, ptr noundef nonnull align 8 dereferenceable(72) %49, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1299, i64 noundef %2726, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %50)
          to label %2735 unwind label %2325, !noalias !ID

.loopexit331:                                     ; preds = %2722, %2719, %2526
  %2732 = phi ptr [ %2521, %2526 ], [ %2725, %2719 ], [ %2725, %2722 ]
  %2733 = phi i64 [ %2523, %2526 ], [ %2723, %2722 ], [ %2720, %2719 ]
  %2734 = phi ptr [ %2522, %2526 ], [ %2725, %2719 ], [ %2725, %2722 ]
  store ptr %2732, ptr %2186, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %49)
  br label %2711

2735:                                             ; preds = %2731
  call void @llvm.lifetime.end.p0(ptr nonnull %50), !noalias !ID
  %2736 = icmp eq i64 %2723, %2529
  br i1 %2736, label %2710, label %2719

.loopexit346:                                     ; preds = %2752, %2501
  %2737 = icmp eq i64 %2348, %2435
  br i1 %2737, label %.loopexit344, label %.lr.ph2430

2738:                                             ; preds = %.lr.ph2430
  %2739 = icmp eq ptr %2440, %2741
  br i1 %2739, label %.loopexit344, label %.lr.ph2430

.lr.ph2430:                                       ; preds = %.loopexit346, %2738
  %2740 = phi ptr [ %2741, %2738 ], [ %2442, %.loopexit346 ]
  %2741 = getelementptr inbounds i8, ptr %2740, i64 -24
  %2742 = load i8, ptr %2741, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2743 = icmp eq i8 %2742, 2
  br i1 %2743, label %2772, label %2738

.preheader345:                                    ; preds = %2501, %2752
  %2744 = phi ptr [ %2745, %2752 ], [ %2440, %2501 ]
  %2745 = getelementptr inbounds nuw i8, ptr %2744, i64 24
  %2746 = load i8, ptr %2744, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2747 = icmp eq i8 %2746, 0
  br i1 %2747, label %2748, label %2752

2748:                                             ; preds = %.preheader345
  %2749 = getelementptr inbounds nuw i8, ptr %2744, i64 1
  %2750 = load i8, ptr %2749, align 1, !range !ID, !noalias !ID, !noundef !ID
  %2751 = icmp eq i8 %2750, -1
  br i1 %2751, label %2752, label %2754

2752:                                             ; preds = %2763, %2757, %2754, %2748, %.preheader345
  %2753 = icmp eq ptr %2745, %2442
  br i1 %2753, label %.loopexit346, label %.preheader345

2754:                                             ; preds = %2748
  %2755 = load ptr, ptr %2201, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2756 = icmp eq ptr %2755, null
  br i1 %2756, label %2752, label %2757

2757:                                             ; preds = %2754
  %2758 = load i32, ptr %2202, align 4, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2759 = getelementptr i8, ptr %2755, i64 56
  %2760 = load i64, ptr %2759, align 8, !noalias !ID, !noundef !ID
  %2761 = zext i32 %2758 to i64
  %2762 = icmp ugt i64 %2760, %2761
  br i1 %2762, label %2763, label %2752

2763:                                             ; preds = %2757
  %2764 = getelementptr i8, ptr %2755, i64 48
  %2765 = load ptr, ptr %2764, align 8, !noalias !ID, !nonnull !ID, !noundef !ID
  %2766 = getelementptr inbounds nuw i8, ptr %2744, i64 8
  %2767 = load i64, ptr %2766, align 8, !noalias !ID, !noundef !ID
  %2768 = zext nneg i8 %2750 to i64
  %2769 = getelementptr inbounds nuw [136 x i8], ptr %2765, i64 %2761
  %2770 = getelementptr inbounds nuw [8 x i8], ptr %2769, i64 %2768
  %2771 = atomicrmw add ptr %2770, i64 %2767 monotonic, align 8, !noalias !ID
  br label %2752

2772:                                             ; preds = %.lr.ph2430
  %2773 = getelementptr i8, ptr %2740, i64 -16
  %2774 = load i64, ptr %2773, align 8, !noalias !ID
  %2775 = icmp ult i64 %2349, %2774
  br i1 %2775, label %2776, label %.loopexit343

2776:                                             ; preds = %2772
  call void @llvm.lifetime.start.p0(ptr nonnull %61)
  %2777 = icmp eq ptr %2350, %2294
  br i1 %2777, label %.loopexit341, label %.preheader340

.loopexit343:                                     ; preds = %2801, %2772
  %2778 = phi ptr [ %2347, %2772 ], [ %2791, %2801 ]
  %2779 = phi ptr [ %2350, %2772 ], [ %2791, %2801 ]
  %2780 = phi i64 [ %2349, %2772 ], [ %2774, %2801 ]
  store ptr %2778, ptr %2186, align 8, !noalias !ID
  br label %.loopexit344

.loopexit344:                                     ; preds = %2738, %.loopexit346, %.loopexit341, %.loopexit343
  %2781 = phi i1 [ false, %.loopexit341 ], [ false, %.loopexit343 ], [ true, %.loopexit346 ], [ true, %2738 ]
  %2782 = phi ptr [ %2799, %.loopexit341 ], [ %2778, %.loopexit343 ], [ %2347, %.loopexit346 ], [ %2347, %2738 ]
  %2783 = phi ptr [ %2800, %.loopexit341 ], [ %2779, %.loopexit343 ], [ %2350, %.loopexit346 ], [ %2350, %2738 ]
  %2784 = phi i64 [ %2798, %.loopexit341 ], [ %2780, %.loopexit343 ], [ %2349, %.loopexit346 ], [ %2349, %2738 ]
  %2785 = icmp eq i64 %2468, 0
  %2786 = or i1 %2158, %2785
  br i1 %2786, label %2804, label %2805

2787:                                             ; preds = %2801
  call void @llvm.lifetime.start.p0(ptr nonnull %61)
  %2788 = icmp eq ptr %2791, %2294
  br i1 %2788, label %.loopexit341, label %.preheader340

.preheader340:                                    ; preds = %2776, %2787
  %2789 = phi ptr [ %2791, %2787 ], [ %2350, %2776 ]
  %2790 = phi i64 [ %2802, %2787 ], [ %2349, %2776 ]
  %2791 = getelementptr inbounds nuw i8, ptr %2789, i64 88
  %2792 = load i64, ptr %2789, align 8, !noalias !ID
  %2793 = getelementptr inbounds nuw i8, ptr %2789, i64 8
  %2794 = load i64, ptr %2793, align 8, !noalias !ID
  %2795 = getelementptr inbounds nuw i8, ptr %2789, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %61, ptr noundef nonnull align 8 dereferenceable(72) %2795, i64 72, i1 false), !noalias !ID
  %2796 = icmp eq i64 %2794, -1
  br i1 %2796, label %.loopexit341, label %2797

2797:                                             ; preds = %.preheader340
  call void @llvm.lifetime.start.p0(ptr nonnull %62), !noalias !ID
  store i64 %2794, ptr %62, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %2203, ptr noundef nonnull align 8 dereferenceable(72) %61, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %61)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1299, i64 noundef %2792, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %62)
          to label %2801 unwind label %2331, !noalias !ID

.loopexit341:                                     ; preds = %.preheader340, %2787, %2776
  %2798 = phi i64 [ %2349, %2776 ], [ %2802, %2787 ], [ %2790, %.preheader340 ]
  %2799 = phi ptr [ %2347, %2776 ], [ %2791, %2787 ], [ %2791, %.preheader340 ]
  %2800 = phi ptr [ %2350, %2776 ], [ %2791, %2787 ], [ %2791, %.preheader340 ]
  store ptr %2799, ptr %2186, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %61)
  br label %.loopexit344

2801:                                             ; preds = %2797
  %2802 = add i64 %2790, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %62), !noalias !ID
  %2803 = icmp eq i64 %2802, %2774
  br i1 %2803, label %.loopexit343, label %2787

2804:                                             ; preds = %2806, %.loopexit344
  br i1 %2161, label %2810, label %2711

2805:                                             ; preds = %.loopexit344
  call void @llvm.lifetime.start.p0(ptr nonnull %60), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %59), !noalias !ID
  store i64 %2468, ptr %59, align 8, !noalias !ID
  store i64 0, ptr %2204, align 8, !noalias !ID
; invoke <purrdf_sparql_eval::governor::GovernorState>::commit_reported_items
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items(ptr noalias nofree noundef nonnull sret([40 x i8]) align 8 captures(address) dereferenceable(40) %60, ptr noundef nonnull align 8 %2156, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %59, i64 noundef 1)
          to label %2806 unwind label %2337, !noalias !ID

2806:                                             ; preds = %2805
  %2807 = load i8, ptr %2205, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2808 = icmp eq i8 %2807, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %59), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %60), !noalias !ID
  br i1 %2808, label %2804, label %2809

2809:                                             ; preds = %2806
  br i1 %2161, label %2826, label %2834

2810:                                             ; preds = %2804
  call void @llvm.lifetime.start.p0(ptr nonnull %70), !noalias !ID
  %2811 = load i64, ptr %2159, align 8, !noalias !ID
  %2812 = icmp eq i64 %2811, -1
  br i1 %2812, label %2814, label %2813

2813:                                             ; preds = %2810
; invoke <purrdf_sparql_eval::governor::GovernorState>::charge_work
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.ID)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %70, ptr noundef nonnull align 8 %2156, i8 noundef 3, i64 noundef %2469, i1 noundef zeroext false)
          to label %2815 unwind label %2337, !noalias !ID

2814:                                             ; preds = %2815, %2810
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !ID
  br i1 %2781, label %2819, label %2821

2815:                                             ; preds = %2813
  %2816 = load i8, ptr %70, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2817 = icmp eq i8 %2816, -1
  br i1 %2817, label %2814, label %2818

2818:                                             ; preds = %2815
  call void @llvm.lifetime.end.p0(ptr nonnull %70), !noalias !ID
  br label %2826

2819:                                             ; preds = %2822, %2814
  %2820 = icmp eq i64 %2470, 0
  br i1 %2820, label %2826, label %2825

2821:                                             ; preds = %2814
  call void @llvm.lifetime.start.p0(ptr nonnull %69), !noalias !ID
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %69, ptr noundef nonnull align 16 dereferenceable(1248) %7)
          to label %2822 unwind label %2337, !noalias !ID

2822:                                             ; preds = %2821
  %2823 = load i8, ptr %69, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2824 = icmp eq i8 %2823, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %69), !noalias !ID
  br i1 %2824, label %2819, label %2826

2825:                                             ; preds = %2819
  call void @llvm.lifetime.start.p0(ptr nonnull %68), !noalias !ID
; invoke <purrdf_sparql_eval::governor::GovernorState>::admit_transient
  invoke void @<purrdf_sparql_eval::governor::GovernorState>::admit_transient(ptr noalias nofree noundef nonnull sret([24 x i8]) align 8 captures(none) dereferenceable(24) %68, ptr noundef nonnull align 8 %2156, i8 noundef 3, i64 noundef %2470)
          to label %2831 unwind label %2337, !noalias !ID

2826:                                             ; preds = %2831, %2822, %2819, %2818, %2809
  %2827 = phi i1 [ false, %2819 ], [ %2833, %2831 ], [ true, %2809 ], [ true, %2818 ], [ true, %2822 ]
  %2828 = icmp ult i64 %2784, %2357
  br i1 %2828, label %2829, label %.loopexit339

2829:                                             ; preds = %2826
  call void @llvm.lifetime.start.p0(ptr nonnull %57)
  %2830 = icmp eq ptr %2783, %2294
  br i1 %2830, label %.loopexit337, label %.preheader336

2831:                                             ; preds = %2825
  %2832 = load i8, ptr %68, align 8, !range !ID, !noalias !ID, !noundef !ID
  %2833 = icmp ne i8 %2832, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %68), !noalias !ID
  br label %2826

2834:                                             ; preds = %2843, %2809
  %2835 = load i64, ptr %82, align 8, !noalias !ID
  %2836 = load ptr, ptr %2038, align 8, !noalias !ID
  %2837 = load i64, ptr %2039, align 8, !noalias !ID
  br label %2914

2838:                                             ; preds = %2859
  call void @llvm.lifetime.start.p0(ptr nonnull %57)
  %2839 = icmp eq ptr %2849, %2294
  br i1 %2839, label %.loopexit337, label %.preheader336

.loopexit339:                                     ; preds = %2859, %2826
  %2840 = phi ptr [ %2782, %2826 ], [ %2849, %2859 ]
  %2841 = phi ptr [ %2783, %2826 ], [ %2849, %2859 ]
  %2842 = phi i64 [ %2784, %2826 ], [ %2357, %2859 ]
  store ptr %2840, ptr %2186, align 8, !noalias !ID
  br label %2843

2843:                                             ; preds = %.loopexit337, %.loopexit339
  %2844 = phi ptr [ %2857, %.loopexit337 ], [ %2840, %.loopexit339 ]
  %2845 = phi i64 [ %2856, %.loopexit337 ], [ %2842, %.loopexit339 ]
  %2846 = phi ptr [ %2858, %.loopexit337 ], [ %2841, %.loopexit339 ]
  br i1 %2827, label %2834, label %2711

.preheader336:                                    ; preds = %2829, %2838
  %2847 = phi ptr [ %2849, %2838 ], [ %2783, %2829 ]
  %2848 = phi i64 [ %2860, %2838 ], [ %2784, %2829 ]
  %2849 = getelementptr inbounds nuw i8, ptr %2847, i64 88
  %2850 = load i64, ptr %2847, align 8, !noalias !ID
  %2851 = getelementptr inbounds nuw i8, ptr %2847, i64 8
  %2852 = load i64, ptr %2851, align 8, !noalias !ID
  %2853 = getelementptr inbounds nuw i8, ptr %2847, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %57, ptr noundef nonnull align 8 dereferenceable(72) %2853, i64 72, i1 false), !noalias !ID
  %2854 = icmp eq i64 %2852, -1
  br i1 %2854, label %.loopexit337, label %2855

2855:                                             ; preds = %.preheader336
  call void @llvm.lifetime.start.p0(ptr nonnull %58), !noalias !ID
  store i64 %2852, ptr %58, align 8, !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %2206, ptr noundef nonnull align 8 dereferenceable(72) %57, i64 72, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %57)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %1299, i64 noundef %2850, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %58)
          to label %2859 unwind label %2329, !noalias !ID

.loopexit337:                                     ; preds = %.preheader336, %2838, %2829
  %2856 = phi i64 [ %2784, %2829 ], [ %2860, %2838 ], [ %2848, %.preheader336 ]
  %2857 = phi ptr [ %2782, %2829 ], [ %2849, %2838 ], [ %2849, %.preheader336 ]
  %2858 = phi ptr [ %2783, %2829 ], [ %2849, %2838 ], [ %2849, %.preheader336 ]
  store ptr %2857, ptr %2186, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %57)
  br label %2843

2859:                                             ; preds = %2855
  %2860 = add i64 %2848, 1
  call void @llvm.lifetime.end.p0(ptr nonnull %58), !noalias !ID
  %2861 = icmp eq i64 %2860, %2357
  br i1 %2861, label %.loopexit339, label %2838

2862:                                             ; preds = %2904, %2717
  %2863 = phi i64 [ %2359, %2717 ], [ %2865, %2904 ]
  %2864 = phi ptr [ %2715, %2717 ], [ %2870, %2904 ]
  %2865 = add i64 %2863, -1
  %2866 = icmp eq ptr %2864, %2718
  br i1 %2866, label %.loopexit329, label %2869

.loopexit329:                                     ; preds = %2904, %2862, %2711
  %2867 = phi ptr [ %2715, %2711 ], [ %2864, %2862 ], [ %2870, %2904 ]
  store ptr %2867, ptr %2045, align 8, !noalias !ID
  %2868 = icmp eq ptr %2352, %2300
  br i1 %2868, label %.loopexit351, label %2346

2869:                                             ; preds = %2862
  %2870 = getelementptr inbounds nuw i8, ptr %2864, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %63), !noalias !ID
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %2220, ptr noundef nonnull align 8 dereferenceable(40) %2864, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %64), !noalias !ID
  store ptr %7, ptr %63, align 8, !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2871 = load i64, ptr %2220, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2872 = icmp eq i64 %2871, 0
  br i1 %2872, label %2873, label %2875

2873:                                             ; preds = %2869
  %2874 = load ptr, ptr %2222, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !align !ID, !noundef !ID
; invoke purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>
  invoke fastcc void @purrdf_sparql_eval::parallel::reintern_portable_row::<purrdf_core::ir::dataset::RdfDataset>(ptr noalias nofree noundef nonnull align 16 captures(address) dereferenceable(96) %64, ptr noalias nofree noundef align 8 dereferenceable(184) %1299, ptr noundef nonnull align 8 %2874, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %2223)
          to label %2876 unwind label %2323, !noalias !ID

2875:                                             ; preds = %2869
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %2221, ptr noundef nonnull align 8 dereferenceable(40) %2864, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !ID
  br label %2887

2876:                                             ; preds = %2873
  %2877 = load i64, ptr %64, align 16, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %63), !noalias !ID
  %2878 = icmp eq i64 %2877, -1
  br i1 %2878, label %2887, label %2879

2879:                                             ; preds = %2876
  store ptr %2870, ptr %2045, align 8, !noalias !ID
  %2880 = load i64, ptr %2221, align 8, !noalias !ID
  %2881 = load ptr, ptr %2224, align 16, !noalias !ID
  %2882 = load i64, ptr %2225, align 8, !noalias !ID
  %2883 = load i8, ptr %2226, align 16, !noalias !ID
  %2884 = load i56, ptr %2227, align 1, !noalias !ID
  %2885 = load i64, ptr %2230, align 8, !noalias !ID
  %2886 = getelementptr inbounds nuw i8, ptr %64, i64 48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %92, ptr noundef nonnull align 16 dereferenceable(48) %2886, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %64), !noalias !ID
  br label %2914

2887:                                             ; preds = %2876, %2875
  %2888 = load i64, ptr %2221, align 8, !noalias !ID
  %2889 = load ptr, ptr %2224, align 16, !noalias !ID
  %2890 = load i64, ptr %2225, align 8, !noalias !ID
  %2891 = load i8, ptr %2226, align 16, !noalias !ID
  %2892 = load i56, ptr %2227, align 1, !noalias !ID
  %2893 = load i64, ptr %2230, align 8, !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %64), !noalias !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2894 = load i64, ptr %2039, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2895 = load i64, ptr %82, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2896 = icmp eq i64 %2894, %2895
  br i1 %2896, label %2897, label %2904

2897:                                             ; preds = %2887
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %82)
          to label %2904 unwind label %2898, !noalias !ID

2898:                                             ; preds = %2897
  %2899 = landingpad { ptr, i32 }
          cleanup
  store ptr %2870, ptr %2045, align 8, !noalias !ID
  %2900 = icmp ugt i64 %2888, 5
  br i1 %2900, label %2901, label %2341

2901:                                             ; preds = %2898
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2889) ]
  %2902 = shl i64 %2888, 3
  %2903 = add i64 %2902, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2889, i64 noundef %2903, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %2341

2904:                                             ; preds = %2897, %2887
  %2905 = load ptr, ptr %2038, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %2906 = getelementptr inbounds nuw [40 x i8], ptr %2905, i64 %2894
  store i64 %2888, ptr %2906, align 8, !noalias !ID
  %2907 = getelementptr inbounds nuw i8, ptr %2906, i64 8
  store ptr %2889, ptr %2907, align 8, !noalias !ID
  %2908 = getelementptr inbounds nuw i8, ptr %2906, i64 16
  store i64 %2890, ptr %2908, align 8, !noalias !ID
  %2909 = getelementptr inbounds nuw i8, ptr %2906, i64 24
  store i8 %2891, ptr %2909, align 8, !noalias !ID
  %2910 = getelementptr inbounds nuw i8, ptr %2906, i64 25
  store i56 %2892, ptr %2910, align 1, !noalias !ID
  %2911 = getelementptr inbounds nuw i8, ptr %2906, i64 32
  store i64 %2893, ptr %2911, align 8, !noalias !ID
  %2912 = add i64 %2894, 1
  store i64 %2912, ptr %2039, align 8, !alias.scope !ID, !noalias !ID
  %2913 = icmp eq i64 %2865, 0
  br i1 %2913, label %.loopexit329, label %2862

2914:                                             ; preds = %2879, %2834, %.loopexit334, %2428
  %2915 = phi i56 [ %2884, %2879 ], [ undef, %.loopexit334 ], [ undef, %2834 ], [ undef, %2428 ]
  %2916 = phi i64 [ %2885, %2879 ], [ undef, %.loopexit334 ], [ undef, %2834 ], [ undef, %2428 ]
  %2917 = phi i64 [ %2880, %2879 ], [ %2707, %.loopexit334 ], [ %2835, %2834 ], [ %2429, %2428 ]
  %2918 = phi ptr [ %2881, %2879 ], [ %2708, %.loopexit334 ], [ %2836, %2834 ], [ %2430, %2428 ]
  %2919 = phi i64 [ %2882, %2879 ], [ %2709, %.loopexit334 ], [ %2837, %2834 ], [ %2431, %2428 ]
  %2920 = phi i8 [ %2883, %2879 ], [ 1, %.loopexit334 ], [ 1, %2834 ], [ 0, %2428 ]
  %2921 = phi i64 [ %2877, %2879 ], [ -1, %.loopexit334 ], [ -1, %2834 ], [ -1, %2428 ]
  %2922 = phi i8 [ 1, %2879 ], [ 0, %.loopexit334 ], [ 0, %2834 ], [ 0, %2428 ]
  %2923 = icmp eq i64 %2296, 0
  br i1 %2923, label %2927, label %2924

2924:                                             ; preds = %2914
  %2925 = shl nuw i64 %2296, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2295, i64 noundef %2925, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %2927

2926:                                             ; preds = %2437
  unreachable

2927:                                             ; preds = %2924, %2914
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %71)
          to label %2928 unwind label %2375, !noalias !ID

2928:                                             ; preds = %2927
  call void @llvm.lifetime.end.p0(ptr nonnull %71), !noalias !ID
  %2929 = icmp eq i64 %2288, 0
  br i1 %2929, label %2932, label %2930

2930:                                             ; preds = %2928
  %2931 = mul nuw i64 %2288, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %2289) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2289, i64 noundef %2931, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %2932

2932:                                             ; preds = %2930, %2928
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2933 = load ptr, ptr %2228, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2934 = icmp eq ptr %2933, null
  br i1 %2934, label %2939, label %2935

2935:                                             ; preds = %2932
  %2936 = atomicrmw sub ptr %2933, i64 1 release, align 8, !noalias !ID
  %2937 = icmp eq i64 %2936, 1
  br i1 %2937, label %2938, label %2939

2938:                                             ; preds = %2935
  fence acquire, !noalias !ID
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2228) #ATTR, !noalias !ID
  br label %2939

2939:                                             ; preds = %2938, %2935, %2932
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2940 = load ptr, ptr %2229, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2941 = icmp eq ptr %2940, null
  br i1 %2941, label %2946, label %2942

2942:                                             ; preds = %2939
  %2943 = atomicrmw sub ptr %2940, i64 1 release, align 8, !noalias !ID
  %2944 = icmp eq i64 %2943, 1
  br i1 %2944, label %2945, label %2946

2945:                                             ; preds = %2942
  fence acquire, !noalias !ID
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2229) #ATTR, !noalias !ID
  br label %2946

2946:                                             ; preds = %2945, %2942, %2939
  call void @llvm.lifetime.end.p0(ptr nonnull %72), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %73)
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %74)
          to label %2947 unwind label %2262, !noalias !ID

2947:                                             ; preds = %2946
  call void @llvm.lifetime.end.p0(ptr nonnull %74), !noalias !ID
  %2948 = atomicrmw sub ptr %2155, i64 1 release, align 8, !noalias !ID
  %2949 = icmp eq i64 %2948, 1
  br i1 %2949, label %2950, label %2951

2950:                                             ; preds = %2947
  fence acquire, !noalias !ID
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %75) #ATTR
          to label %2951 unwind label %2084, !noalias !ID

2951:                                             ; preds = %2950, %2947
  call void @llvm.lifetime.end.p0(ptr nonnull %75), !noalias !ID
  br label %2142

2952:                                             ; preds = %2954, %2142
  call void @llvm.lifetime.end.p0(ptr nonnull %79), !noalias !ID
  %2953 = trunc nuw i8 %2150 to i1
  br i1 %2953, label %2955, label %2100

2954:                                             ; preds = %2142
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %79)
          to label %2952 unwind label %2096, !noalias !ID

2955:                                             ; preds = %2952
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2956 = load ptr, ptr %2038, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %2957 = load i64, ptr %2039, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID), !noalias !ID
  %2958 = icmp eq i64 %2957, 0
  br i1 %2958, label %.loopexit318, label %.preheader317

.preheader317:                                    ; preds = %2955, %2969
  %2959 = phi i64 [ %2961, %2969 ], [ 0, %2955 ]
  %2960 = getelementptr inbounds nuw [40 x i8], ptr %2956, i64 %2959
  %2961 = add nuw nsw i64 %2959, 1
  %2962 = load i64, ptr %2960, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2963 = icmp ugt i64 %2962, 5
  br i1 %2963, label %2964, label %2969

2964:                                             ; preds = %.preheader317
  %2965 = getelementptr i8, ptr %2960, i64 8
  %2966 = load ptr, ptr %2965, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %2967 = shl i64 %2962, 3
  %2968 = add i64 %2967, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2966, i64 noundef %2968, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %2969

2969:                                             ; preds = %2964, %.preheader317
  %2970 = icmp eq i64 %2961, %2957
  br i1 %2970, label %.loopexit318, label %.preheader317

.loopexit318:                                     ; preds = %2969, %2955
  %2971 = load i64, ptr %82, align 8, !alias.scope !ID, !noalias !ID
  %2972 = icmp eq i64 %2971, 0
  br i1 %2972, label %2100, label %2973

2973:                                             ; preds = %.loopexit318
  %2974 = mul nuw i64 %2971, 40
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %2956, i64 noundef %2974, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %2100

2975:                                             ; preds = %2387, %2384, %2381
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %2976 = load ptr, ptr %2229, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %2977 = icmp eq ptr %2976, null
  br i1 %2977, label %2280, label %2978

2978:                                             ; preds = %2975
  %2979 = atomicrmw sub ptr %2976, i64 1 release, align 8, !noalias !ID
  %2980 = icmp eq i64 %2979, 1
  br i1 %2980, label %2981, label %2280

2981:                                             ; preds = %2978
  fence acquire, !noalias !ID
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %2229) #ATTR, !noalias !ID
  br label %2280

2982:                                             ; preds = %2261, %2255, %2084
  %2983 = phi { ptr, i32 } [ %2087, %2084 ], [ %2258, %2255 ], [ %2258, %2261 ]
  %2984 = phi i8 [ %2086, %2084 ], [ %2257, %2255 ], [ %2257, %2261 ]
  %2985 = phi i8 [ %2085, %2084 ], [ %2256, %2255 ], [ %2256, %2261 ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(32) %79) #ATTR
          to label %2091 unwind label %2152, !noalias !ID

2986:                                             ; preds = %2091, %2070
  %2987 = phi { ptr, i32 } [ %2094, %2091 ], [ %2071, %2070 ]
  %2988 = phi i8 [ %2093, %2091 ], [ 1, %2070 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %82) #ATTR, !noalias !ID
  br label %2989

2989:                                             ; preds = %2986, %2091
  %2990 = phi i8 [ %2093, %2091 ], [ %2988, %2986 ]
  %2991 = phi { ptr, i32 } [ %2094, %2091 ], [ %2987, %2986 ]
  %2992 = trunc nuw i8 %2990 to i1
  br i1 %2992, label %3027, label %3536

2993:                                             ; preds = %2029
  %2994 = landingpad { ptr, i32 }
          cleanup
  br label %3027

2995:                                             ; preds = %2100, %2088, %2029
  %2996 = phi i56 [ undef, %2088 ], [ %2143, %2100 ], [ %2033, %2029 ]
  %2997 = phi i64 [ undef, %2088 ], [ %2144, %2100 ], [ %2035, %2029 ]
  %2998 = phi i64 [ %2089, %2088 ], [ %2145, %2100 ], [ %2024, %2029 ]
  %2999 = phi ptr [ %2090, %2088 ], [ %2146, %2100 ], [ %2026, %2029 ]
  %3000 = phi i64 [ %2082, %2088 ], [ %2147, %2100 ], [ %2028, %2029 ]
  %3001 = phi i8 [ 2, %2088 ], [ %2148, %2100 ], [ %2031, %2029 ]
  %3002 = phi i64 [ -1, %2088 ], [ %2149, %2100 ], [ %2021, %2029 ]
  %3003 = icmp eq i64 %1977, 0
  br i1 %3003, label %._crit_edge2433, label %.lr.ph2432

3004:                                             ; preds = %.lr.ph2432
  %3005 = icmp eq i64 %3008, %1977
  br i1 %3005, label %._crit_edge2433, label %.lr.ph2432

.lr.ph2432:                                       ; preds = %2995, %3004
  %3006 = phi i64 [ %3008, %3004 ], [ 0, %2995 ]
  %3007 = getelementptr inbounds nuw [160 x i8], ptr %1978, i64 %3006
  %3008 = add nuw nsw i64 %3006, 1
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %3007)
          to label %3004 unwind label %3012, !noalias !ID

3009:                                             ; preds = %.lr.ph2435
  %3010 = add i64 %3015, 1
  %3011 = icmp eq i64 %3010, %1977
  br i1 %3011, label %._crit_edge2436, label %.lr.ph2435

3012:                                             ; preds = %.lr.ph2432
  %3013 = landingpad { ptr, i32 }
          cleanup
  %3014 = icmp eq i64 %3008, %1977
  br i1 %3014, label %._crit_edge2436, label %.lr.ph2435

.lr.ph2435:                                       ; preds = %3012, %3009
  %3015 = phi i64 [ %3010, %3009 ], [ %3008, %3012 ]
  %3016 = getelementptr inbounds nuw [160 x i8], ptr %1978, i64 %3015
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef readonly align 8 dereferenceable(160) %3016) #ATTR
          to label %3009 unwind label %3017, !noalias !ID

3017:                                             ; preds = %.lr.ph2435
  %3018 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

._crit_edge2436:                                  ; preds = %3009, %3012
  %3019 = icmp eq i64 %1989, 0
  br i1 %3019, label %3536, label %3020

3020:                                             ; preds = %._crit_edge2436
  %3021 = mul nuw i64 %1989, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1978, i64 noundef %3021, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %3536

._crit_edge2433:                                  ; preds = %3004, %2995
  %3022 = icmp eq i64 %1989, 0
  br i1 %3022, label %3029, label %3023

3023:                                             ; preds = %._crit_edge2433
  %3024 = mul nuw i64 %1989, 160
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1978, i64 noundef %3024, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %3029

3025:                                             ; preds = %.loopexit355
  %3026 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %124) #ATTR
          to label %3027 unwind label %2152

3027:                                             ; preds = %3025, %2993, %2989
  %3028 = phi { ptr, i32 } [ %2994, %2993 ], [ %2991, %2989 ], [ %3026, %3025 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %91) #ATTR
          to label %3536 unwind label %2152, !noalias !ID

3029:                                             ; preds = %3023, %._crit_edge2433, %2100
  %3030 = phi i56 [ %2996, %._crit_edge2433 ], [ %2996, %3023 ], [ %2143, %2100 ]
  %3031 = phi i64 [ %2997, %._crit_edge2433 ], [ %2997, %3023 ], [ %2144, %2100 ]
  %3032 = phi i64 [ %2998, %._crit_edge2433 ], [ %2998, %3023 ], [ %2145, %2100 ]
  %3033 = phi ptr [ %2999, %._crit_edge2433 ], [ %2999, %3023 ], [ %2146, %2100 ]
  %3034 = phi i64 [ %3000, %._crit_edge2433 ], [ %3000, %3023 ], [ %2147, %2100 ]
  %3035 = phi i8 [ %3001, %._crit_edge2433 ], [ %3001, %3023 ], [ %2148, %2100 ]
  %3036 = phi i64 [ %3002, %._crit_edge2433 ], [ %3002, %3023 ], [ %2149, %2100 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %91), !noalias !ID
  %3037 = icmp eq i64 %3036, -1
  br i1 %3037, label %3039, label %3038

3038:                                             ; preds = %3029
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %122, ptr noundef nonnull align 16 dereferenceable(48) %92, i64 48, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %92)
  call void @llvm.lifetime.end.p0(ptr nonnull %120)
  call void @llvm.lifetime.end.p0(ptr nonnull %121)
  br label %3098

3039:                                             ; preds = %3029, %2314
  %3040 = phi i8 [ 2, %2314 ], [ %3035, %3029 ]
  %3041 = phi i64 [ %2309, %2314 ], [ %3034, %3029 ]
  %3042 = phi ptr [ %2308, %2314 ], [ %3033, %3029 ]
  %3043 = phi i64 [ %2307, %2314 ], [ %3032, %3029 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %92)
  call void @llvm.lifetime.start.p0(ptr nonnull %90), !noalias !ID
  store i64 %3043, ptr %90, align 8, !noalias !ID
  %3044 = getelementptr inbounds nuw i8, ptr %90, i64 8
  store ptr %3042, ptr %3044, align 8, !noalias !ID
  %3045 = getelementptr inbounds nuw i8, ptr %90, i64 16
  store i64 %3041, ptr %3045, align 8, !noalias !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %39)
  %3046 = load ptr, ptr %1306, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3047 = icmp eq ptr %3046, null
  br i1 %3047, label %3058, label %3048

3048:                                             ; preds = %3039
  %3049 = getelementptr inbounds nuw i8, ptr %3046, i64 296
  %3050 = load atomic i32, ptr %3049 acquire, align 4, !noalias !ID
  %3051 = icmp eq i32 %3050, 0
  br i1 %3051, label %3052, label %3056

3052:                                             ; preds = %3048
  %3053 = getelementptr inbounds nuw i8, ptr %3046, i64 272
  %3054 = load i8, ptr %3053, align 8, !noalias !ID
  %3055 = getelementptr inbounds nuw i8, ptr %3046, i64 273
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %39, ptr noundef nonnull align 1 dereferenceable(23) %3055, i64 23, i1 false), !noalias !ID
  br label %3056

3056:                                             ; preds = %3052, %3048
  %3057 = phi i8 [ %3054, %3052 ], [ -1, %3048 ]
  switch i8 %3040, label %3062 [
    i8 2, label %3079
    i8 0, label %3061
  ]

3058:                                             ; preds = %3039
  %3059 = icmp eq i8 %3040, 2
  %3060 = and i1 %3059, %2011
  br label %3079

3061:                                             ; preds = %3076, %3064, %3056
  br label %3079

3062:                                             ; preds = %3056
  %3063 = icmp eq i8 %3057, -1
  br i1 %3063, label %3079, label %3064

3064:                                             ; preds = %3062
  %3065 = load i8, ptr %884, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3066 = icmp eq i8 %3065, 2
  br i1 %3066, label %3067, label %3061

3067:                                             ; preds = %3064
  %3068 = getelementptr inbounds nuw i8, ptr %7, i64 696
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3069 = load ptr, ptr %3068, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %38), !noalias !ID
  store i8 %3057, ptr %38, align 8, !noalias !ID
  %3070 = getelementptr inbounds nuw i8, ptr %38, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %3070, ptr noundef nonnull align 1 dereferenceable(23) %39, i64 23, i1 false), !noalias !ID
  %3071 = getelementptr inbounds nuw i8, ptr %3069, i64 40
  %3072 = load atomic i32, ptr %3071 acquire, align 4, !noalias !ID
  %3073 = icmp eq i32 %3072, 0
  br i1 %3073, label %3076, label %3074, !prof !ID

3074:                                             ; preds = %3067
  %3075 = getelementptr inbounds nuw i8, ptr %3069, i64 16
; invoke <std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !>
  invoke fastcc void @<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::initialize::<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::get_or_init<<std::sync::once_lock::OnceLock<purrdf_core::governor::TrippedGovernor>>::try_insert::{closure#0}>::{closure#0}, !> (.llvm.ID)(ptr noundef nonnull align 8 %3075, ptr noundef nonnull align 8 %38)
          to label %3076 unwind label %3077, !noalias !ID

3076:                                             ; preds = %3074, %3067
  call void @llvm.lifetime.end.p0(ptr nonnull %38), !noalias !ID
  br label %3061

3077:                                             ; preds = %3074
  %3078 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %90) #ATTR, !noalias !ID
  br label %3536

3079:                                             ; preds = %3062, %3061, %3058, %3056
  %3080 = phi i8 [ -1, %3058 ], [ -1, %3062 ], [ %3057, %3061 ], [ %3057, %3056 ]
  %3081 = phi i1 [ %3060, %3058 ], [ false, %3062 ], [ false, %3061 ], [ %2011, %3056 ]
  %3082 = icmp eq i8 %3080, -1
  %3083 = select i1 %3081, i1 %3082, i1 false
  call void @llvm.lifetime.end.p0(ptr nonnull %39)
  call void @llvm.lifetime.end.p0(ptr nonnull %90), !noalias !ID
  %3084 = zext i1 %3083 to i8
  call void @llvm.lifetime.end.p0(ptr nonnull %120)
  call void @llvm.lifetime.end.p0(ptr nonnull %121)
  br label %3117

3085:                                             ; preds = %3091, %3089
  %3086 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

3087:                                             ; preds = %1953, %1842
  %3088 = phi { ptr, i32 } [ %1843, %1842 ], [ %1954, %1953 ]
  br i1 %1783, label %3091, label %3089

3089:                                             ; preds = %3087, %1930, %1903
  %3090 = phi { ptr, i32 } [ %3088, %3087 ], [ %1904, %1903 ], [ %1931, %1930 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(176) %120) #ATTR
          to label %3536 unwind label %3085, !noalias !ID

3091:                                             ; preds = %3087
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(24) %121) #ATTR
          to label %3536 unwind label %3085, !noalias !ID

3092:                                             ; preds = %1776
  call void @llvm.lifetime.start.p0(ptr nonnull %128)
  %3093 = load i64, ptr %153, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %128, ptr noundef nonnull align 8 dereferenceable(16) %476, i64 16, i1 false)
  store i64 0, ptr %153, align 8
  store ptr inttoptr (i64 8 to ptr), ptr %476, align 8
  store i64 0, ptr %478, align 8
  %3094 = icmp ult i64 %1760, 230584300921369396
  call void @llvm.assume(i1 %3094)
  %3095 = icmp samesign ugt i64 %3093, %1760
  br i1 %3095, label %3133, label %3137

3096:                                             ; preds = %1943
  call void @llvm.lifetime.end.p0(ptr nonnull %120)
  call void @llvm.lifetime.end.p0(ptr nonnull %121)
  %3097 = icmp eq i64 %1950, -1
  br i1 %3097, label %3117, label %3098

3098:                                             ; preds = %3096, %3038
  %3099 = phi i64 [ %3036, %3038 ], [ %1950, %3096 ]
  %3100 = phi i64 [ %3032, %3038 ], [ %1949, %3096 ]
  %3101 = phi ptr [ %3033, %3038 ], [ %1948, %3096 ]
  %3102 = phi i64 [ %3034, %3038 ], [ %1947, %3096 ]
  %3103 = phi i64 [ %3031, %3038 ], [ %1946, %3096 ]
  %3104 = phi i8 [ %3035, %3038 ], [ %1945, %3096 ]
  %3105 = phi i56 [ %3030, %3038 ], [ %1944, %3096 ]
  %3106 = zext i56 %3105 to i64
  %3107 = shl nuw i64 %3106, 8
  %3108 = zext i8 %3104 to i64
  %3109 = or disjoint i64 %3107, %3108
  %3110 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %3110, ptr noundef nonnull align 16 dereferenceable(48) %122, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %122)
  %3111 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %3099, ptr %3111, align 16
  %3112 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %3100, ptr %3112, align 8
  %3113 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store ptr %3101, ptr %3113, align 16
  %3114 = getelementptr inbounds nuw i8, ptr %0, i64 40
  store i64 %3102, ptr %3114, align 8
  %3115 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %3109, ptr %3115, align 16
  %3116 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i64 %3103, ptr %3116, align 8
  store i64 1, ptr %0, align 16
  br label %3132

3117:                                             ; preds = %3096, %3079
  %3118 = phi i64 [ %3043, %3079 ], [ %1949, %3096 ]
  %3119 = phi ptr [ %3042, %3079 ], [ %1948, %3096 ]
  %3120 = phi i64 [ %3041, %3079 ], [ %1947, %3096 ]
  %3121 = phi i64 [ %2010, %3079 ], [ %1946, %3096 ]
  %3122 = phi i8 [ %3084, %3079 ], [ %1945, %3096 ]
  %3123 = zext i8 %3122 to i64
  call void @llvm.lifetime.end.p0(ptr nonnull %122)
  br label %3124

3124:                                             ; preds = %3165, %3117
  %3125 = phi i64 [ %3147, %3165 ], [ %3118, %3117 ]
  %3126 = phi ptr [ %3149, %3165 ], [ %3119, %3117 ]
  %3127 = phi i64 [ %3151, %3165 ], [ %3120, %3117 ]
  %3128 = phi i64 [ %3155, %3165 ], [ %3123, %3117 ]
  %3129 = phi i64 [ %3154, %3165 ], [ %3121, %3117 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %129)
  store i64 %3125, ptr %129, align 8
  %3130 = getelementptr inbounds nuw i8, ptr %129, i64 8
  store ptr %3126, ptr %3130, align 8
  %3131 = getelementptr inbounds nuw i8, ptr %129, i64 16
  store i64 %3127, ptr %3131, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %119)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %119, ptr noundef nonnull align 8 dereferenceable(32) %132, i64 32, i1 false)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::absorb_worker_witnesses::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::absorb_worker_witnesses::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 16 dereferenceable(1248) %7, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %119)
          to label %3172 unwind label %3170

3132:                                             ; preds = %3156, %3098
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %132)
          to label %3286 unwind label %1772

3133:                                             ; preds = %3092
  call void @llvm.lifetime.start.p0(ptr nonnull %126)
  store i64 %3093, ptr %126, align 8
  %3134 = getelementptr inbounds nuw i8, ptr %126, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %3134, ptr noundef nonnull align 8 dereferenceable(16) %128, i64 16, i1 false)
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %126)
  call void @llvm.lifetime.end.p0(ptr nonnull %126)
  %3135 = getelementptr inbounds nuw i8, ptr %127, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %3135, align 8
  %3136 = getelementptr inbounds nuw i8, ptr %127, i64 16
  store i64 0, ptr %3136, align 8
  br label %3139

3137:                                             ; preds = %3092
  %3138 = getelementptr inbounds nuw i8, ptr %127, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %3138, ptr noundef nonnull align 8 dereferenceable(16) %128, i64 16, i1 false)
  br label %3139

3139:                                             ; preds = %3137, %3133
  %3140 = phi i64 [ 0, %3133 ], [ %3093, %3137 ]
  store i64 %3140, ptr %127, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %125)
  %3141 = getelementptr inbounds nuw i8, ptr %138, i64 160
  %3142 = load i8, ptr %3141, align 8, !range !ID, !noundef !ID
; invoke <purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#9}>
  invoke fastcc void @<purrdf_sparql_eval::row_checkpoint::ItemLedger>::commit_into::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::parallel::MintedRow, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::ItemLedger; 1]>, purrdf_sparql_eval::modifier::eval_group_with<purrdf_core::ir::dataset::RdfDataset, &[purrdf_sparql_algebra::ast::Variable]>::{closure#9}>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %125, i8 %3142, ptr noalias nofree noundef align 16 dereferenceable(1248) %7, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %124, ptr noalias nofree noundef align 8 captures(address) dereferenceable(176) %123, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %127)
          to label %3143 unwind label %1774

3143:                                             ; preds = %3139
  %3144 = load i64, ptr %125, align 16, !range !ID, !noundef !ID
  %3145 = icmp eq i64 %3144, -1
  %3146 = getelementptr inbounds nuw i8, ptr %125, i64 8
  %3147 = load i64, ptr %3146, align 8
  %3148 = getelementptr inbounds nuw i8, ptr %125, i64 16
  %3149 = load ptr, ptr %3148, align 16
  %3150 = getelementptr inbounds nuw i8, ptr %125, i64 24
  %3151 = load i64, ptr %3150, align 8
  %3152 = getelementptr inbounds nuw i8, ptr %125, i64 32
  %3153 = getelementptr inbounds nuw i8, ptr %125, i64 40
  %3154 = load i64, ptr %3153, align 8
  %3155 = load i64, ptr %3152, align 16
  br i1 %3145, label %3165, label %3156

3156:                                             ; preds = %3143
  %3157 = getelementptr inbounds nuw i8, ptr %125, i64 48
  %3158 = getelementptr inbounds nuw i8, ptr %0, i64 64
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(48) %3158, ptr noundef nonnull align 16 dereferenceable(48) %3157, i64 48, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  %3159 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %3144, ptr %3159, align 16
  %3160 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i64 %3147, ptr %3160, align 8
  %3161 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store ptr %3149, ptr %3161, align 16
  %3162 = getelementptr inbounds nuw i8, ptr %0, i64 40
  store i64 %3151, ptr %3162, align 8
  %3163 = getelementptr inbounds nuw i8, ptr %0, i64 48
  store i64 %3155, ptr %3163, align 16
  %3164 = getelementptr inbounds nuw i8, ptr %0, i64 56
  store i64 %3154, ptr %3164, align 8
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %128)
  br label %3132

3165:                                             ; preds = %3143
  call void @llvm.lifetime.end.p0(ptr nonnull %125)
  call void @llvm.lifetime.end.p0(ptr nonnull %128)
  br label %3124

3166:                                             ; preds = %3297, %3293, %3215, %3170, %3168
  %3167 = phi { ptr, i32 } [ %3295, %3297 ], [ %3216, %3215 ], [ %3295, %3293 ], [ %3169, %3168 ], [ %3171, %3170 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %129) #ATTR
  br label %1325

3168:                                             ; preds = %3195
  %3169 = landingpad { ptr, i32 }
          cleanup
  br label %3166

3170:                                             ; preds = %3194, %3124
  %3171 = landingpad { ptr, i32 }
          cleanup
  br label %3166

3172:                                             ; preds = %3124
  call void @llvm.lifetime.end.p0(ptr nonnull %119)
  %3173 = trunc i64 %3128 to i1
  br i1 %3173, label %3174, label %.loopexit316

3174:                                             ; preds = %3172
  %3175 = load i64, ptr %878, align 8, !noundef !ID
  %3176 = icmp ugt i64 %3129, %3175
  br i1 %3176, label %3194, label %3183, !prof !ID

.loopexit316:                                     ; preds = %3253, %3183, %3172
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %140, ptr noundef nonnull align 8 dereferenceable(24) %129, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %129)
  call void @llvm.lifetime.end.p0(ptr nonnull %132)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3177 = load ptr, ptr %137, align 8, !alias.scope !ID, !noundef !ID
  %3178 = icmp eq ptr %3177, null
  br i1 %3178, label %3301, label %3179

3179:                                             ; preds = %.loopexit316
  %3180 = atomicrmw sub ptr %3177, i64 1 release, align 8, !noalias !ID
  %3181 = icmp eq i64 %3180, 1
  br i1 %3181, label %3182, label %3301

3182:                                             ; preds = %3179
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %137) #ATTR
          to label %3301 unwind label %1312, !inline_history !ID

3183:                                             ; preds = %3174
  %3184 = load ptr, ptr %879, align 8, !nonnull !ID, !noundef !ID
  %3185 = getelementptr inbounds nuw [72 x i8], ptr %3184, i64 %3175
  %3186 = icmp samesign eq i64 %3129, %3175
  br i1 %3186, label %.loopexit316, label %3187

3187:                                             ; preds = %3183
  %3188 = getelementptr inbounds nuw [72 x i8], ptr %3184, i64 %3129
  %3189 = getelementptr inbounds nuw i8, ptr %117, i64 8
  %3190 = getelementptr inbounds nuw i8, ptr %117, i64 16
  %3191 = getelementptr inbounds nuw i8, ptr %141, i64 8
  %3192 = getelementptr inbounds nuw i8, ptr %141, i64 16
  %3193 = getelementptr inbounds nuw i8, ptr %118, i64 8
  br label %3195

3194:                                             ; preds = %3174
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %3129, i64 noundef %3175, i64 noundef %3175, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.535) #ATTR
          to label %1111 unwind label %3170

3195:                                             ; preds = %3253, %3187
  %3196 = phi ptr [ %3188, %3187 ], [ %3197, %3253 ]
  %3197 = getelementptr inbounds nuw i8, ptr %3196, i64 72
  %3198 = load i64, ptr %142, align 8, !noundef !ID
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem(ptr noalias nofree noundef align 8 captures(none) dereferenceable(40) %117, i64 noundef %3198)
          to label %3199 unwind label %3168

3199:                                             ; preds = %3195
  %3200 = load i64, ptr %117, align 8, !range !ID, !noundef !ID
  %3201 = icmp ugt i64 %3200, 5
  %3202 = load ptr, ptr %3189, align 8, !nonnull !ID
  %3203 = select i1 %3201, ptr %3202, ptr %3189
  %3204 = load i64, ptr %3190, align 8
  %3205 = select i1 %3201, i64 %3204, i64 %3200
  %3206 = add i64 %3205, -1
  %3207 = load i64, ptr %158, align 8, !noundef !ID
  %3208 = icmp ugt i64 %3207, %3206
  br i1 %3208, label %3209, label %3210, !prof !ID

3209:                                             ; preds = %3199
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef 0, i64 noundef %3207, i64 noundef %3206, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.532) #ATTR
          to label %1111 unwind label %3290

3210:                                             ; preds = %3199
  %3211 = load i64, ptr %3196, align 8, !range !ID, !noundef !ID
  %3212 = add i64 %3211, -1
  %3213 = icmp ugt i64 %3212, 4
  %3214 = getelementptr inbounds nuw i8, ptr %3196, i64 8
  br i1 %3213, label %3217, label %3222

3215:                                             ; preds = %.loopexit315
  %3216 = landingpad { ptr, i32 }
          cleanup
  br label %3166

3217:                                             ; preds = %3210
  %3218 = load ptr, ptr %3214, align 8, !nonnull !ID, !noundef !ID
  %3219 = getelementptr inbounds nuw i8, ptr %3196, i64 16
  %3220 = load i64, ptr %3219, align 8, !noundef !ID
  %3221 = add i64 %3220, -1
  br label %3222

3222:                                             ; preds = %3217, %3210
  %3223 = phi i64 [ %3221, %3217 ], [ %3212, %3210 ]
  %3224 = phi ptr [ %3218, %3217 ], [ %3214, %3210 ]
  %3225 = icmp eq i64 %3207, %3223
  br i1 %3225, label %3228, label %3226, !prof !ID

3226:                                             ; preds = %3222
; invoke core::slice::copy_from_slice_impl::len_mismatch_fail
  invoke void @core::slice::copy_from_slice_impl::len_mismatch_fail(i64 noundef range(i64 0, 1152921504606846976) %3207, i64 noundef range(i64 0, 1152921504606846976) %3223, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.533) #ATTR
          to label %3227 unwind label %3290

3227:                                             ; preds = %3226
  unreachable

3228:                                             ; preds = %3222
  %3229 = shl nuw nsw i64 %3207, 3
  call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 4 %3203, ptr nonnull readonly align 4 %3224, i64 %3229, i1 false)
  %3230 = load ptr, ptr %3191, align 8, !nonnull !ID, !noundef !ID
  %3231 = load i64, ptr %3192, align 8, !noundef !ID
  %3232 = call i64 @llvm.umin.i64(i64 %6, i64 %3231)
  %3233 = icmp eq i64 %3232, 0
  br i1 %3233, label %.loopexit315, label %3234

3234:                                             ; preds = %3228
  %3235 = getelementptr inbounds nuw i8, ptr %3196, i64 56
  %3236 = getelementptr inbounds nuw i8, ptr %3196, i64 64
  br label %3237

3237:                                             ; preds = %3279, %3234
  %3238 = phi i64 [ 0, %3234 ], [ %3239, %3279 ]
  %3239 = add nuw nsw i64 %3238, 1
  %3240 = getelementptr inbounds nuw [120 x i8], ptr %5, i64 %3238
  %3241 = getelementptr inbounds nuw [24 x i8], ptr %3230, i64 %3238
  %3242 = getelementptr inbounds nuw i8, ptr %3240, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %118)
  %3243 = getelementptr inbounds nuw i8, ptr %3241, i64 8
  %3244 = load ptr, ptr %3243, align 8, !nonnull !ID, !noundef !ID
  %3245 = getelementptr inbounds nuw i8, ptr %3241, i64 16
  %3246 = load i64, ptr %3245, align 8, !noundef !ID
  %3247 = load ptr, ptr %3235, align 8, !nonnull !ID, !noundef !ID
  %3248 = load i64, ptr %3236, align 8, !noundef !ID
  %3249 = load ptr, ptr %476, align 8, !nonnull !ID, !noundef !ID
  %3250 = load i64, ptr %478, align 8, !noundef !ID
  %3251 = load ptr, ptr %150, align 8, !nonnull !ID, !noundef !ID
  %3252 = getelementptr inbounds nuw i8, ptr %3251, i64 16
; invoke purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>
  invoke fastcc void @purrdf_sparql_eval::modifier::eval_aggregate::<purrdf_core::ir::dataset::RdfDataset, ()>(ptr noalias nofree noundef align 16 captures(address) dereferenceable(96) %118, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(104) %3242, ptr noalias nofree noundef nonnull align 8 %3244, i64 noundef %3246, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3247, i64 noundef %3248, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) %3249, i64 noundef %3250, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %3252, ptr noalias nofree noundef align 16 dereferenceable(1248) %7)
          to label %3255 unwind label %3287

.loopexit315:                                     ; preds = %3279, %3228
; invoke <alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::push_mut
  invoke fastcc void @<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::push_mut(ptr noalias nofree noundef align 8 dereferenceable(24) %129, ptr noalias nofree noundef align 8 captures(address) dereferenceable(40) %117)
          to label %3253 unwind label %3215

3253:                                             ; preds = %.loopexit315
  %3254 = icmp eq ptr %3197, %3185
  br i1 %3254, label %.loopexit316, label %3195

3255:                                             ; preds = %3237
  %3256 = load i64, ptr %118, align 16, !range !ID, !noundef !ID
  %3257 = icmp eq i64 %3256, -1
  %3258 = load <2 x i32>, ptr %3193, align 8
  br i1 %3257, label %3270, label %3259

3259:                                             ; preds = %3255
  %3260 = getelementptr inbounds nuw i8, ptr %118, i64 16
  %3261 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(80) %3261, ptr noundef nonnull align 16 dereferenceable(80) %3260, i64 80, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %118)
  %3262 = getelementptr inbounds nuw i8, ptr %0, i64 16
  store i64 %3256, ptr %3262, align 16
  %3263 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store <2 x i32> %3258, ptr %3263, align 8
  store i64 1, ptr %0, align 16
  %3264 = load i64, ptr %117, align 8, !range !ID, !noundef !ID
  %3265 = icmp ugt i64 %3264, 5
  br i1 %3265, label %3266, label %3285

3266:                                             ; preds = %3259
  %3267 = load ptr, ptr %3189, align 8, !nonnull !ID, !noundef !ID
  %3268 = shl i64 %3264, 3
  %3269 = add i64 %3268, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3267, i64 noundef %3269, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %3285

3270:                                             ; preds = %3255
  call void @llvm.lifetime.end.p0(ptr nonnull %118)
  %3271 = load i64, ptr %158, align 8, !noundef !ID
  %3272 = add i64 %3271, %3238
  %3273 = load i64, ptr %117, align 8, !range !ID, !noundef !ID
  %3274 = icmp ugt i64 %3273, 5
  %3275 = load i64, ptr %3190, align 8
  %3276 = select i1 %3274, i64 %3275, i64 %3273
  %3277 = add i64 %3276, -1
  %3278 = icmp ult i64 %3272, %3277
  br i1 %3278, label %3279, label %3284

3279:                                             ; preds = %3270
  %3280 = load ptr, ptr %3189, align 8, !nonnull !ID
  %3281 = select i1 %3274, ptr %3280, ptr %3189
  %3282 = getelementptr inbounds nuw [8 x i8], ptr %3281, i64 %3272
  store <2 x i32> %3258, ptr %3282, align 4
  %3283 = icmp eq i64 %3239, %3232
  br i1 %3283, label %.loopexit315, label %3237

3284:                                             ; preds = %3270
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %3272, i64 noundef %3277, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.HASH.534) #ATTR
          to label %1111 unwind label %3290

3285:                                             ; preds = %3266, %3259
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %129)
  call void @llvm.lifetime.end.p0(ptr nonnull %129)
  br label %3286

3286:                                             ; preds = %3285, %3132
  call void @llvm.lifetime.end.p0(ptr nonnull %132)
  br label %3529

3287:                                             ; preds = %3237
  %3288 = landingpad { ptr, i32 }
          cleanup
  %3289 = load i64, ptr %117, align 8, !range !ID
  br label %3293

3290:                                             ; preds = %3284, %3226, %3209
  %3291 = phi i64 [ %3200, %3209 ], [ %3200, %3226 ], [ %3273, %3284 ]
  %3292 = landingpad { ptr, i32 }
          cleanup
  br label %3293

3293:                                             ; preds = %3290, %3287
  %3294 = phi i64 [ %3289, %3287 ], [ %3291, %3290 ]
  %3295 = phi { ptr, i32 } [ %3288, %3287 ], [ %3292, %3290 ]
  %3296 = icmp ugt i64 %3294, 5
  br i1 %3296, label %3297, label %3166

3297:                                             ; preds = %3293
  %3298 = load ptr, ptr %3189, align 8, !nonnull !ID, !noundef !ID
  %3299 = shl i64 %3294, 3
  %3300 = add i64 %3299, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3298, i64 noundef %3300, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %3166

3301:                                             ; preds = %3182, %3179, %.loopexit316
  call void @llvm.lifetime.end.p0(ptr nonnull %137)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(168) %138)
          to label %3302 unwind label %1074

3302:                                             ; preds = %3301
  call void @llvm.lifetime.end.p0(ptr nonnull %138)
  call void @llvm.lifetime.end.p0(ptr nonnull %139)
  br label %1292

3303:                                             ; preds = %3335
  %3304 = landingpad { ptr, i32 }
          cleanup
  br label %1070

3305:                                             ; preds = %1292
  %3306 = getelementptr inbounds nuw i8, ptr %1294, i64 16
  %3307 = load i8, ptr %3306, align 8, !noalias !ID
  %3308 = icmp eq i8 %3307, -1
  br i1 %3308, label %3312, label %3309

3309:                                             ; preds = %3305
  %3310 = getelementptr inbounds nuw i8, ptr %1294, i64 17
  call void @llvm.lifetime.start.p0(ptr nonnull %112)
  store i8 %3307, ptr %112, align 8
  %3311 = getelementptr inbounds nuw i8, ptr %112, i64 1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 1 dereferenceable(23) %3311, ptr noundef nonnull align 1 dereferenceable(23) %3310, i64 23, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %111)
; invoke <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::barred_at(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %111, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %112, ptr noundef nonnull %193)
          to label %3336 unwind label %3527

3312:                                             ; preds = %3305, %1292
  call void @llvm.lifetime.start.p0(ptr nonnull %110)
  call void @llvm.lifetime.start.p0(ptr nonnull %108)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %108, ptr noundef nonnull align 8 dereferenceable(24) %140, i64 24, i1 false)
  %3313 = getelementptr inbounds nuw i8, ptr %108, i64 24
  store ptr %193, ptr %3313, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3314 = load i64, ptr %109, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3315 = icmp eq i64 %3314, -1
  br i1 %3315, label %3317, label %3316

3316:                                             ; preds = %3312
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %110, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %108, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %109)
  br label %3319

3317:                                             ; preds = %3312
  %3318 = getelementptr inbounds nuw i8, ptr %110, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %3318, ptr noundef nonnull readonly align 8 dereferenceable(32) %108, i64 32, i1 false), !alias.scope !ID, !noalias !ID
  store i64 -1, ptr %110, align 8, !alias.scope !ID, !noalias !ID
  br label %3319

3319:                                             ; preds = %3317, %3316
  %3320 = getelementptr inbounds nuw i8, ptr %109, i64 72
  %3321 = load i64, ptr %3320, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3322 = icmp ugt i64 %3321, 5
  br i1 %3322, label %3323, label %3328

3323:                                             ; preds = %3319
  %3324 = getelementptr inbounds nuw i8, ptr %109, i64 80
  %3325 = load ptr, ptr %3324, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %3326 = mul i64 %3321, 3
  %3327 = add i64 %3326, -3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3325, i64 noundef %3327, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %3328

3328:                                             ; preds = %3323, %3319
  %3329 = getelementptr inbounds nuw i8, ptr %109, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3330 = load ptr, ptr %3329, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3331 = icmp eq ptr %3330, null
  br i1 %3331, label %3395, label %3332

3332:                                             ; preds = %3328
  %3333 = atomicrmw sub ptr %3330, i64 1 release, align 8, !noalias !ID
  %3334 = icmp eq i64 %3333, 1
  br i1 %3334, label %3335, label %3395

3335:                                             ; preds = %3332
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %3329) #ATTR
          to label %3395 unwind label %3303

3336:                                             ; preds = %3309
  %3337 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %3337, ptr noundef nonnull align 8 dereferenceable(96) %111, i64 96, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %111)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %112)
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %140)
  br label %3338

3338:                                             ; preds = %3541, %3336, %1275
  %3339 = phi i8 [ 1, %3541 ], [ 0, %3336 ], [ 1, %1275 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %140)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3340 = getelementptr inbounds nuw i8, ptr %141, i64 8
  %3341 = load ptr, ptr %3340, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %3342 = getelementptr inbounds nuw i8, ptr %141, i64 16
  %3343 = load i64, ptr %3342, align 8, !alias.scope !ID, !noundef !ID
  %3344 = icmp eq i64 %3343, 0
  br i1 %3344, label %._crit_edge2445, label %.lr.ph2444

3345:                                             ; preds = %.lr.ph2444
  %3346 = icmp eq i64 %3349, %3343
  br i1 %3346, label %._crit_edge2445, label %.lr.ph2444

.lr.ph2444:                                       ; preds = %3338, %3345
  %3347 = phi i64 [ %3349, %3345 ], [ 0, %3338 ]
  %3348 = getelementptr inbounds nuw [24 x i8], ptr %3341, i64 %3347
  %3349 = add nuw nsw i64 %3347, 1
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %3348)
          to label %3345 unwind label %3353, !noalias !ID

3350:                                             ; preds = %.lr.ph2447
  %3351 = add i64 %3356, 1
  %3352 = icmp eq i64 %3351, %3343
  br i1 %3352, label %._crit_edge2448, label %.lr.ph2447

3353:                                             ; preds = %.lr.ph2444
  %3354 = landingpad { ptr, i32 }
          cleanup
  %3355 = icmp eq i64 %3349, %3343
  br i1 %3355, label %._crit_edge2448, label %.lr.ph2447

.lr.ph2447:                                       ; preds = %3353, %3350
  %3356 = phi i64 [ %3351, %3350 ], [ %3349, %3353 ]
  %3357 = getelementptr inbounds nuw [24 x i8], ptr %3341, i64 %3356
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %3357) #ATTR
          to label %3350 unwind label %3358, !noalias !ID

3358:                                             ; preds = %.lr.ph2447
  %3359 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

._crit_edge2448:                                  ; preds = %3350, %3353
  %3360 = load i64, ptr %141, align 8, !alias.scope !ID
  %3361 = icmp eq i64 %3360, 0
  br i1 %3361, label %865, label %3362

3362:                                             ; preds = %._crit_edge2448
  %3363 = mul nuw i64 %3360, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3341, i64 noundef %3363, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %865

._crit_edge2445:                                  ; preds = %3345, %3338
  %3364 = load i64, ptr %141, align 8, !alias.scope !ID
  %3365 = icmp eq i64 %3364, 0
  br i1 %3365, label %3542, label %3366

3366:                                             ; preds = %._crit_edge2445
  %3367 = mul nuw i64 %3364, 24
  %3368 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3369 = call i64 @llvm.umin.i64(i64 %3367, i64 9223372036854775807)
  %3370 = call i64 @llvm.ssub.sat.i64(i64 %3368, i64 %3369)
  store i64 %3370, ptr %202, align 8, !noalias !ID
  %3371 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %3372 = load i64, ptr %3371, align 8, !noalias !ID, !noundef !ID
  %3373 = icmp slt i64 %3370, %3372
  br i1 %3373, label %3374, label %.preheader2551

3374:                                             ; preds = %3366
  store i64 %3370, ptr %3371, align 8, !noalias !ID
  br label %.preheader2551

.preheader2551:                                   ; preds = %3374, %3366
  br label %3375

3375:                                             ; preds = %.preheader2551, %3378
  %3376 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3377 = icmp slt i64 %3376, 0
  br i1 %3377, label %3378, label %__rustc::__rust_dealloc (.exit301)

3378:                                             ; preds = %3375
  %3379 = add nsw i64 %3376, 1
  %3380 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3376, i64 %3379 acq_rel acquire, align 8, !noalias !ID
  %3381 = extractvalue { i64, i1 } %3380, 1
  br i1 %3381, label %3382, label %3375

3382:                                             ; preds = %3378
  %3383 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3369 monotonic, align 8, !noalias !ID
  %3384 = call i64 @llvm.ssub.sat.i64(i64 %3383, i64 %3369)
  %3385 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3386

3386:                                             ; preds = %3389, %3382
  %3387 = phi i64 [ %3385, %3382 ], [ %3392, %3389 ]
  %3388 = icmp slt i64 %3384, %3387
  br i1 %3388, label %3389, label %3393

3389:                                             ; preds = %3386
  %3390 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3387, i64 %3384 monotonic monotonic, align 8, !noalias !ID
  %3391 = extractvalue { i64, i1 } %3390, 1
  %3392 = extractvalue { i64, i1 } %3390, 0
  br i1 %3391, label %3393, label %3386

3393:                                             ; preds = %3389, %3386
  %3394 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit301)

__rustc::__rust_dealloc (.exit301): ; preds = %3375, %3393
  call void @free(ptr noundef nonnull %3341) #ATTR, !noalias !ID
  br label %3542

3395:                                             ; preds = %3335, %3332, %3328
  call void @llvm.lifetime.end.p0(ptr nonnull %108)
  %3396 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %3396, ptr noundef nonnull align 8 dereferenceable(96) %110, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %110)
  call void @llvm.lifetime.end.p0(ptr nonnull %140)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3397 = getelementptr inbounds nuw i8, ptr %141, i64 8
  %3398 = load ptr, ptr %3397, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %3399 = getelementptr inbounds nuw i8, ptr %141, i64 16
  %3400 = load i64, ptr %3399, align 8, !alias.scope !ID, !noundef !ID
  %3401 = icmp eq i64 %3400, 0
  br i1 %3401, label %._crit_edge2439, label %.lr.ph2438

3402:                                             ; preds = %.lr.ph2438
  %3403 = icmp eq i64 %3406, %3400
  br i1 %3403, label %._crit_edge2439, label %.lr.ph2438

.lr.ph2438:                                       ; preds = %3395, %3402
  %3404 = phi i64 [ %3406, %3402 ], [ 0, %3395 ]
  %3405 = getelementptr inbounds nuw [24 x i8], ptr %3398, i64 %3404
  %3406 = add nuw nsw i64 %3404, 1
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %3405)
          to label %3402 unwind label %3410, !noalias !ID

3407:                                             ; preds = %.lr.ph2441
  %3408 = add i64 %3413, 1
  %3409 = icmp eq i64 %3408, %3400
  br i1 %3409, label %._crit_edge2442, label %.lr.ph2441

3410:                                             ; preds = %.lr.ph2438
  %3411 = landingpad { ptr, i32 }
          cleanup
  %3412 = icmp eq i64 %3406, %3400
  br i1 %3412, label %._crit_edge2442, label %.lr.ph2441

.lr.ph2441:                                       ; preds = %3410, %3407
  %3413 = phi i64 [ %3408, %3407 ], [ %3406, %3410 ]
  %3414 = getelementptr inbounds nuw [24 x i8], ptr %3398, i64 %3413
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::vm::Linked<purrdf_core::ir::term::TermId>>>(ptr noalias nofree noundef readonly align 8 dereferenceable(24) %3414) #ATTR
          to label %3407 unwind label %3415, !noalias !ID

3415:                                             ; preds = %.lr.ph2441
  %3416 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #ATTR, !noalias !ID
  unreachable

._crit_edge2442:                                  ; preds = %3407, %3410
  %3417 = load i64, ptr %141, align 8, !alias.scope !ID
  %3418 = icmp eq i64 %3417, 0
  br i1 %3418, label %865, label %3419

3419:                                             ; preds = %._crit_edge2442
  %3420 = mul nuw i64 %3417, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3398, i64 noundef %3420, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %865

._crit_edge2439:                                  ; preds = %3402, %3395
  %3421 = load i64, ptr %141, align 8, !alias.scope !ID
  %3422 = icmp eq i64 %3421, 0
  br i1 %3422, label %3425, label %3423

3423:                                             ; preds = %._crit_edge2439
  %3424 = mul nuw i64 %3421, 24
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3398, i64 noundef %3424, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %3425

3425:                                             ; preds = %3423, %._crit_edge2439
  call void @llvm.lifetime.end.p0(ptr nonnull %141)
  call void @llvm.lifetime.end.p0(ptr nonnull %142)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3426 = getelementptr inbounds nuw i8, ptr %144, i64 8
  %3427 = load ptr, ptr %3426, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %3428 = getelementptr inbounds nuw i8, ptr %144, i64 16
  %3429 = load i64, ptr %3428, align 8, !alias.scope !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3430 = icmp eq i64 %3429, 0
  br i1 %3430, label %.loopexit314, label %.preheader313

.preheader313:                                    ; preds = %3425
  %3431 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  br label %3432

3432:                                             ; preds = %.preheader313, %3504
  %3433 = phi i64 [ %3435, %3504 ], [ 0, %.preheader313 ]
  %3434 = getelementptr inbounds nuw [72 x i8], ptr %3427, i64 %3433
  %3435 = add nuw nsw i64 %3433, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3436 = load i64, ptr %3434, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3437 = icmp ugt i64 %3436, 5
  br i1 %3437, label %3438, label %3470

3438:                                             ; preds = %3432
  %3439 = getelementptr inbounds nuw i8, ptr %3434, i64 8
  %3440 = load ptr, ptr %3439, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %3441 = shl i64 %3436, 3
  %3442 = add i64 %3441, -8
  %3443 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3444 = call i64 @llvm.umin.i64(i64 %3442, i64 9223372036854775807)
  %3445 = call i64 @llvm.ssub.sat.i64(i64 %3443, i64 %3444)
  store i64 %3445, ptr %202, align 8, !noalias !ID
  %3446 = load i64, ptr %3431, align 8, !noalias !ID, !noundef !ID
  %3447 = icmp slt i64 %3445, %3446
  br i1 %3447, label %3448, label %.preheader2555

3448:                                             ; preds = %3438
  store i64 %3445, ptr %3431, align 8, !noalias !ID
  br label %.preheader2555

.preheader2555:                                   ; preds = %3448, %3438
  br label %3449

3449:                                             ; preds = %.preheader2555, %3452
  %3450 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3451 = icmp slt i64 %3450, 0
  br i1 %3451, label %3452, label %__rustc::__rust_dealloc (.exit302)

3452:                                             ; preds = %3449
  %3453 = add nsw i64 %3450, 1
  %3454 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3450, i64 %3453 acq_rel acquire, align 8, !noalias !ID
  %3455 = extractvalue { i64, i1 } %3454, 1
  br i1 %3455, label %3456, label %3449

3456:                                             ; preds = %3452
  %3457 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3444 monotonic, align 8, !noalias !ID
  %3458 = call i64 @llvm.ssub.sat.i64(i64 %3457, i64 %3444)
  %3459 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3460

3460:                                             ; preds = %3463, %3456
  %3461 = phi i64 [ %3459, %3456 ], [ %3466, %3463 ]
  %3462 = icmp slt i64 %3458, %3461
  br i1 %3462, label %3463, label %3467

3463:                                             ; preds = %3460
  %3464 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3461, i64 %3458 monotonic monotonic, align 8, !noalias !ID
  %3465 = extractvalue { i64, i1 } %3464, 1
  %3466 = extractvalue { i64, i1 } %3464, 0
  br i1 %3465, label %3467, label %3460

3467:                                             ; preds = %3463, %3460
  %3468 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit302)

__rustc::__rust_dealloc (.exit302): ; preds = %3449, %3467
  %3469 = icmp ne i64 %3442, 0
  call void @llvm.assume(i1 %3469), !noalias !ID
  call void @free(ptr noundef nonnull %3440) #ATTR, !noalias !ID
  br label %3470

3470:                                             ; preds = %__rustc::__rust_dealloc (.exit302), %3432
  %3471 = getelementptr inbounds nuw i8, ptr %3434, i64 48
  %3472 = load i64, ptr %3471, align 8, !alias.scope !ID, !noalias !ID
  %3473 = icmp eq i64 %3472, 0
  br i1 %3473, label %3504, label %3474

3474:                                             ; preds = %3470
  %3475 = getelementptr inbounds nuw i8, ptr %3434, i64 56
  %3476 = load ptr, ptr %3475, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %3477 = shl nuw i64 %3472, 3
  %3478 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3479 = call i64 @llvm.umin.i64(i64 %3477, i64 9223372036854775807)
  %3480 = call i64 @llvm.ssub.sat.i64(i64 %3478, i64 %3479)
  store i64 %3480, ptr %202, align 8, !noalias !ID
  %3481 = load i64, ptr %3431, align 8, !noalias !ID, !noundef !ID
  %3482 = icmp slt i64 %3480, %3481
  br i1 %3482, label %3483, label %.preheader2554

3483:                                             ; preds = %3474
  store i64 %3480, ptr %3431, align 8, !noalias !ID
  br label %.preheader2554

.preheader2554:                                   ; preds = %3483, %3474
  br label %3484

3484:                                             ; preds = %.preheader2554, %3487
  %3485 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3486 = icmp slt i64 %3485, 0
  br i1 %3486, label %3487, label %__rustc::__rust_dealloc (.exit303)

3487:                                             ; preds = %3484
  %3488 = add nsw i64 %3485, 1
  %3489 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3485, i64 %3488 acq_rel acquire, align 8, !noalias !ID
  %3490 = extractvalue { i64, i1 } %3489, 1
  br i1 %3490, label %3491, label %3484

3491:                                             ; preds = %3487
  %3492 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3479 monotonic, align 8, !noalias !ID
  %3493 = call i64 @llvm.ssub.sat.i64(i64 %3492, i64 %3479)
  %3494 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3495

3495:                                             ; preds = %3498, %3491
  %3496 = phi i64 [ %3494, %3491 ], [ %3501, %3498 ]
  %3497 = icmp slt i64 %3493, %3496
  br i1 %3497, label %3498, label %3502

3498:                                             ; preds = %3495
  %3499 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3496, i64 %3493 monotonic monotonic, align 8, !noalias !ID
  %3500 = extractvalue { i64, i1 } %3499, 1
  %3501 = extractvalue { i64, i1 } %3499, 0
  br i1 %3500, label %3502, label %3495

3502:                                             ; preds = %3498, %3495
  %3503 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit303)

__rustc::__rust_dealloc (.exit303): ; preds = %3484, %3502
  call void @free(ptr noundef nonnull %3476) #ATTR, !noalias !ID
  br label %3504

3504:                                             ; preds = %__rustc::__rust_dealloc (.exit303), %3470
  %3505 = icmp eq i64 %3435, %3429
  br i1 %3505, label %.loopexit314, label %3432

.loopexit314:                                     ; preds = %3504, %3425
  %3506 = load i64, ptr %144, align 8, !alias.scope !ID
  %3507 = icmp eq i64 %3506, 0
  br i1 %3507, label %3510, label %3508

3508:                                             ; preds = %.loopexit314
  %3509 = mul nuw i64 %3506, 72
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3427, i64 noundef %3509, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR, !noalias !ID
  br label %3510

3510:                                             ; preds = %3508, %.loopexit314
  call void @llvm.lifetime.end.p0(ptr nonnull %144)
  call void @llvm.lifetime.end.p0(ptr nonnull %149)
  br i1 %423, label %3512, label %3511

3511:                                             ; preds = %3510
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %475, i64 noundef %422, i64 noundef range(i64 1, -9223372036854775807) 8) #ATTR
  br label %3512

3512:                                             ; preds = %3511, %3510
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3513 = load ptr, ptr %150, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %3514 = atomicrmw sub ptr %3513, i64 1 release, align 8, !noalias !ID
  %3515 = icmp eq i64 %3514, 1
  br i1 %3515, label %3516, label %3525

3516:                                             ; preds = %3512
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %150) #ATTR
          to label %3525 unwind label %3521

3517:                                             ; preds = %3521, %409, %402
  %3518 = phi i1 [ %3522, %3521 ], [ %403, %409 ], [ %403, %402 ]
  %3519 = phi i8 [ %3523, %3521 ], [ %404, %409 ], [ %404, %402 ]
  %3520 = phi { ptr, i32 } [ %3524, %3521 ], [ %405, %409 ], [ %405, %402 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %153) #ATTR
          to label %303 unwind label %1290

3521:                                             ; preds = %3685, %3516
  %3522 = phi i1 [ true, %3685 ], [ false, %3516 ]
  %3523 = phi i8 [ %3339, %3685 ], [ 0, %3516 ]
  %3524 = landingpad { ptr, i32 }
          cleanup
  br label %3517

3525:                                             ; preds = %3516, %3512
  call void @llvm.lifetime.end.p0(ptr nonnull %150)
; call core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %153)
  call void @llvm.lifetime.end.p0(ptr nonnull %153)
  call void @llvm.lifetime.end.p0(ptr nonnull %157)
  call void @llvm.lifetime.end.p0(ptr nonnull %158)
  br label %3526

3526:                                             ; preds = %3989, %3525, %294, %__rustc::__rust_alloc (.exit288)
  call void @llvm.lifetime.end.p0(ptr nonnull %159)
  ret void

3527:                                             ; preds = %3309
  %3528 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %140) #ATTR
  br label %1070

3529:                                             ; preds = %3286, %1742
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3530 = load ptr, ptr %137, align 8, !alias.scope !ID, !noundef !ID
  %3531 = icmp eq ptr %3530, null
  br i1 %3531, label %3540, label %3532

3532:                                             ; preds = %3529
  %3533 = atomicrmw sub ptr %3530, i64 1 release, align 8, !noalias !ID
  %3534 = icmp eq i64 %3533, 1
  br i1 %3534, label %3535, label %3540

3535:                                             ; preds = %3532
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::scratch::ScratchInterner>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %137) #ATTR
          to label %3540 unwind label %1312, !inline_history !ID

3536:                                             ; preds = %3091, %3089, %3077, %3027, %3020, %._crit_edge2436, %2989, %1951, %1774
  %3537 = phi { ptr, i32 } [ %3028, %3027 ], [ %1952, %1951 ], [ %1775, %1774 ], [ %3090, %3089 ], [ %2991, %2989 ], [ %3013, %._crit_edge2436 ], [ %3078, %3077 ], [ %3013, %3020 ], [ %3088, %3091 ]
; invoke core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::small::SmallVec<[purrdf_sparql_eval::witness::RelationWitness; 1]>>(ptr noalias nofree noundef align 8 dereferenceable(32) %132) #ATTR
          to label %1325 unwind label %1290

3538:                                             ; preds = %1748
  %3539 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::parallel::MintedRow>>(ptr noalias nofree noundef align 8 dereferenceable(24) %124) #ATTR
          to label %1325 unwind label %1290

3540:                                             ; preds = %3535, %3532, %3529
  call void @llvm.lifetime.end.p0(ptr nonnull %137)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(168) %138)
          to label %3541 unwind label %1074

3541:                                             ; preds = %3540
  call void @llvm.lifetime.end.p0(ptr nonnull %138)
  call void @llvm.lifetime.end.p0(ptr nonnull %139)
  br label %3338

3542:                                             ; preds = %__rustc::__rust_dealloc (.exit301), %._crit_edge2445
  call void @llvm.lifetime.end.p0(ptr nonnull %141)
  call void @llvm.lifetime.end.p0(ptr nonnull %142)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3543 = load ptr, ptr %879, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %3544 = load i64, ptr %878, align 8, !alias.scope !ID, !noundef !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3545 = icmp eq i64 %3544, 0
  br i1 %3545, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %3542
  %3546 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  br label %3547

3547:                                             ; preds = %.preheader, %3619
  %3548 = phi i64 [ %3550, %3619 ], [ 0, %.preheader ]
  %3549 = getelementptr inbounds nuw [72 x i8], ptr %3543, i64 %3548
  %3550 = add nuw nsw i64 %3548, 1
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3551 = load i64, ptr %3549, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3552 = icmp ugt i64 %3551, 5
  br i1 %3552, label %3553, label %3585

3553:                                             ; preds = %3547
  %3554 = getelementptr inbounds nuw i8, ptr %3549, i64 8
  %3555 = load ptr, ptr %3554, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %3556 = shl i64 %3551, 3
  %3557 = add i64 %3556, -8
  %3558 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3559 = call i64 @llvm.umin.i64(i64 %3557, i64 9223372036854775807)
  %3560 = call i64 @llvm.ssub.sat.i64(i64 %3558, i64 %3559)
  store i64 %3560, ptr %202, align 8, !noalias !ID
  %3561 = load i64, ptr %3546, align 8, !noalias !ID, !noundef !ID
  %3562 = icmp slt i64 %3560, %3561
  br i1 %3562, label %3563, label %.preheader2550

3563:                                             ; preds = %3553
  store i64 %3560, ptr %3546, align 8, !noalias !ID
  br label %.preheader2550

.preheader2550:                                   ; preds = %3563, %3553
  br label %3564

3564:                                             ; preds = %.preheader2550, %3567
  %3565 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3566 = icmp slt i64 %3565, 0
  br i1 %3566, label %3567, label %__rustc::__rust_dealloc (.exit304)

3567:                                             ; preds = %3564
  %3568 = add nsw i64 %3565, 1
  %3569 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3565, i64 %3568 acq_rel acquire, align 8, !noalias !ID
  %3570 = extractvalue { i64, i1 } %3569, 1
  br i1 %3570, label %3571, label %3564

3571:                                             ; preds = %3567
  %3572 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3559 monotonic, align 8, !noalias !ID
  %3573 = call i64 @llvm.ssub.sat.i64(i64 %3572, i64 %3559)
  %3574 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3575

3575:                                             ; preds = %3578, %3571
  %3576 = phi i64 [ %3574, %3571 ], [ %3581, %3578 ]
  %3577 = icmp slt i64 %3573, %3576
  br i1 %3577, label %3578, label %3582

3578:                                             ; preds = %3575
  %3579 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3576, i64 %3573 monotonic monotonic, align 8, !noalias !ID
  %3580 = extractvalue { i64, i1 } %3579, 1
  %3581 = extractvalue { i64, i1 } %3579, 0
  br i1 %3580, label %3582, label %3575

3582:                                             ; preds = %3578, %3575
  %3583 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit304)

__rustc::__rust_dealloc (.exit304): ; preds = %3564, %3582
  %3584 = icmp ne i64 %3557, 0
  call void @llvm.assume(i1 %3584), !noalias !ID
  call void @free(ptr noundef nonnull %3555) #ATTR, !noalias !ID
  br label %3585

3585:                                             ; preds = %__rustc::__rust_dealloc (.exit304), %3547
  %3586 = getelementptr inbounds nuw i8, ptr %3549, i64 48
  %3587 = load i64, ptr %3586, align 8, !alias.scope !ID, !noalias !ID
  %3588 = icmp eq i64 %3587, 0
  br i1 %3588, label %3619, label %3589

3589:                                             ; preds = %3585
  %3590 = getelementptr inbounds nuw i8, ptr %3549, i64 56
  %3591 = load ptr, ptr %3590, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %3592 = shl nuw i64 %3587, 3
  %3593 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3594 = call i64 @llvm.umin.i64(i64 %3592, i64 9223372036854775807)
  %3595 = call i64 @llvm.ssub.sat.i64(i64 %3593, i64 %3594)
  store i64 %3595, ptr %202, align 8, !noalias !ID
  %3596 = load i64, ptr %3546, align 8, !noalias !ID, !noundef !ID
  %3597 = icmp slt i64 %3595, %3596
  br i1 %3597, label %3598, label %.preheader2549

3598:                                             ; preds = %3589
  store i64 %3595, ptr %3546, align 8, !noalias !ID
  br label %.preheader2549

.preheader2549:                                   ; preds = %3598, %3589
  br label %3599

3599:                                             ; preds = %.preheader2549, %3602
  %3600 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3601 = icmp slt i64 %3600, 0
  br i1 %3601, label %3602, label %__rustc::__rust_dealloc (.exit305)

3602:                                             ; preds = %3599
  %3603 = add nsw i64 %3600, 1
  %3604 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3600, i64 %3603 acq_rel acquire, align 8, !noalias !ID
  %3605 = extractvalue { i64, i1 } %3604, 1
  br i1 %3605, label %3606, label %3599

3606:                                             ; preds = %3602
  %3607 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3594 monotonic, align 8, !noalias !ID
  %3608 = call i64 @llvm.ssub.sat.i64(i64 %3607, i64 %3594)
  %3609 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3610

3610:                                             ; preds = %3613, %3606
  %3611 = phi i64 [ %3609, %3606 ], [ %3616, %3613 ]
  %3612 = icmp slt i64 %3608, %3611
  br i1 %3612, label %3613, label %3617

3613:                                             ; preds = %3610
  %3614 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3611, i64 %3608 monotonic monotonic, align 8, !noalias !ID
  %3615 = extractvalue { i64, i1 } %3614, 1
  %3616 = extractvalue { i64, i1 } %3614, 0
  br i1 %3615, label %3617, label %3610

3617:                                             ; preds = %3613, %3610
  %3618 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit305)

__rustc::__rust_dealloc (.exit305): ; preds = %3599, %3617
  call void @free(ptr noundef nonnull %3591) #ATTR, !noalias !ID
  br label %3619

3619:                                             ; preds = %__rustc::__rust_dealloc (.exit305), %3585
  %3620 = icmp eq i64 %3550, %3544
  br i1 %3620, label %.loopexit, label %3547

.loopexit:                                        ; preds = %3619, %3542
  %3621 = load i64, ptr %144, align 8, !alias.scope !ID
  %3622 = icmp eq i64 %3621, 0
  br i1 %3622, label %3652, label %3623

3623:                                             ; preds = %.loopexit
  %3624 = mul nuw i64 %3621, 72
  %3625 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3626 = call i64 @llvm.umin.i64(i64 %3624, i64 9223372036854775807)
  %3627 = call i64 @llvm.ssub.sat.i64(i64 %3625, i64 %3626)
  store i64 %3627, ptr %202, align 8, !noalias !ID
  %3628 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %3629 = load i64, ptr %3628, align 8, !noalias !ID, !noundef !ID
  %3630 = icmp slt i64 %3627, %3629
  br i1 %3630, label %3631, label %.preheader2548

3631:                                             ; preds = %3623
  store i64 %3627, ptr %3628, align 8, !noalias !ID
  br label %.preheader2548

.preheader2548:                                   ; preds = %3631, %3623
  br label %3632

3632:                                             ; preds = %.preheader2548, %3635
  %3633 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3634 = icmp slt i64 %3633, 0
  br i1 %3634, label %3635, label %__rustc::__rust_dealloc (.exit306)

3635:                                             ; preds = %3632
  %3636 = add nsw i64 %3633, 1
  %3637 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3633, i64 %3636 acq_rel acquire, align 8, !noalias !ID
  %3638 = extractvalue { i64, i1 } %3637, 1
  br i1 %3638, label %3639, label %3632

3639:                                             ; preds = %3635
  %3640 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3626 monotonic, align 8, !noalias !ID
  %3641 = call i64 @llvm.ssub.sat.i64(i64 %3640, i64 %3626)
  %3642 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3643

3643:                                             ; preds = %3646, %3639
  %3644 = phi i64 [ %3642, %3639 ], [ %3649, %3646 ]
  %3645 = icmp slt i64 %3641, %3644
  br i1 %3645, label %3646, label %3650

3646:                                             ; preds = %3643
  %3647 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3644, i64 %3641 monotonic monotonic, align 8, !noalias !ID
  %3648 = extractvalue { i64, i1 } %3647, 1
  %3649 = extractvalue { i64, i1 } %3647, 0
  br i1 %3648, label %3650, label %3643

3650:                                             ; preds = %3646, %3643
  %3651 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit306)

__rustc::__rust_dealloc (.exit306): ; preds = %3632, %3650
  call void @free(ptr noundef nonnull %3543) #ATTR, !noalias !ID
  br label %3652

3652:                                             ; preds = %__rustc::__rust_dealloc (.exit306), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %144)
  call void @llvm.lifetime.end.p0(ptr nonnull %149)
  br i1 %423, label %3681, label %3653

3653:                                             ; preds = %3652
  %3654 = load i64, ptr %202, align 8, !noundef !ID
  %3655 = call i64 @llvm.umin.i64(i64 %422, i64 9223372036854775807)
  %3656 = call i64 @llvm.ssub.sat.i64(i64 %3654, i64 %3655)
  store i64 %3656, ptr %202, align 8
  %3657 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %3658 = load i64, ptr %3657, align 8, !noundef !ID
  %3659 = icmp slt i64 %3656, %3658
  br i1 %3659, label %3660, label %.preheader2547

3660:                                             ; preds = %3653
  store i64 %3656, ptr %3657, align 8
  br label %.preheader2547

.preheader2547:                                   ; preds = %3660, %3653
  br label %3661

3661:                                             ; preds = %.preheader2547, %3664
  %3662 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8
  %3663 = icmp slt i64 %3662, 0
  br i1 %3663, label %3664, label %__rustc::__rust_dealloc (.exit307)

3664:                                             ; preds = %3661
  %3665 = add nsw i64 %3662, 1
  %3666 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3662, i64 %3665 acq_rel acquire, align 8
  %3667 = extractvalue { i64, i1 } %3666, 1
  br i1 %3667, label %3668, label %3661

3668:                                             ; preds = %3664
  %3669 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3655 monotonic, align 8
  %3670 = call i64 @llvm.ssub.sat.i64(i64 %3669, i64 %3655)
  %3671 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %3672

3672:                                             ; preds = %3675, %3668
  %3673 = phi i64 [ %3671, %3668 ], [ %3678, %3675 ]
  %3674 = icmp slt i64 %3670, %3673
  br i1 %3674, label %3675, label %3679

3675:                                             ; preds = %3672
  %3676 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3673, i64 %3670 monotonic monotonic, align 8
  %3677 = extractvalue { i64, i1 } %3676, 1
  %3678 = extractvalue { i64, i1 } %3676, 0
  br i1 %3677, label %3679, label %3672

3679:                                             ; preds = %3675, %3672
  %3680 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit307)

__rustc::__rust_dealloc (.exit307): ; preds = %3661, %3679
  call void @free(ptr noundef nonnull %475) #ATTR
  br label %3681

3681:                                             ; preds = %__rustc::__rust_dealloc (.exit307), %3652
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3682 = load ptr, ptr %150, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %3683 = atomicrmw sub ptr %3682, i64 1 release, align 8, !noalias !ID
  %3684 = icmp eq i64 %3683, 1
  br i1 %3684, label %3685, label %3686

3685:                                             ; preds = %3681
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %150) #ATTR
          to label %3686 unwind label %3521

3686:                                             ; preds = %3685, %3681
  call void @llvm.lifetime.end.p0(ptr nonnull %150)
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %153)
          to label %3687 unwind label %304

3687:                                             ; preds = %3686
  call void @llvm.lifetime.end.p0(ptr nonnull %153)
  %3688 = getelementptr inbounds nuw i8, ptr %109, i64 72
  %3689 = load i64, ptr %3688, align 8, !range !ID, !noundef !ID
  %3690 = icmp ugt i64 %3689, 5
  br i1 %3690, label %3691, label %3724

3691:                                             ; preds = %3687
  %3692 = getelementptr inbounds nuw i8, ptr %109, i64 80
  %3693 = load ptr, ptr %3692, align 8, !nonnull !ID, !noundef !ID
  %3694 = mul i64 %3689, 3
  %3695 = add i64 %3694, -3
  %3696 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3697 = call i64 @llvm.umin.i64(i64 %3695, i64 9223372036854775807)
  %3698 = call i64 @llvm.ssub.sat.i64(i64 %3696, i64 %3697)
  store i64 %3698, ptr %202, align 8, !noalias !ID
  %3699 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %3700 = load i64, ptr %3699, align 8, !noalias !ID, !noundef !ID
  %3701 = icmp slt i64 %3698, %3700
  br i1 %3701, label %3702, label %.preheader2546

3702:                                             ; preds = %3691
  store i64 %3698, ptr %3699, align 8, !noalias !ID
  br label %.preheader2546

.preheader2546:                                   ; preds = %3702, %3691
  br label %3703

3703:                                             ; preds = %.preheader2546, %3706
  %3704 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3705 = icmp slt i64 %3704, 0
  br i1 %3705, label %3706, label %__rustc::__rust_dealloc (.exit308)

3706:                                             ; preds = %3703
  %3707 = add nsw i64 %3704, 1
  %3708 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3704, i64 %3707 acq_rel acquire, align 8, !noalias !ID
  %3709 = extractvalue { i64, i1 } %3708, 1
  br i1 %3709, label %3710, label %3703

3710:                                             ; preds = %3706
  %3711 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3697 monotonic, align 8, !noalias !ID
  %3712 = call i64 @llvm.ssub.sat.i64(i64 %3711, i64 %3697)
  %3713 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3714

3714:                                             ; preds = %3717, %3710
  %3715 = phi i64 [ %3713, %3710 ], [ %3720, %3717 ]
  %3716 = icmp slt i64 %3712, %3715
  br i1 %3716, label %3717, label %3721

3717:                                             ; preds = %3714
  %3718 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3715, i64 %3712 monotonic monotonic, align 8, !noalias !ID
  %3719 = extractvalue { i64, i1 } %3718, 1
  %3720 = extractvalue { i64, i1 } %3718, 0
  br i1 %3719, label %3721, label %3714

3721:                                             ; preds = %3717, %3714
  %3722 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit308)

__rustc::__rust_dealloc (.exit308): ; preds = %3703, %3721
  %3723 = icmp ne i64 %3695, 0
  call void @llvm.assume(i1 %3723), !noalias !ID
  call void @free(ptr noundef nonnull %3693) #ATTR, !noalias !ID
  br label %3724

3724:                                             ; preds = %__rustc::__rust_dealloc (.exit308), %3687
  %3725 = load i64, ptr %109, align 8, !range !ID, !noundef !ID
  %3726 = icmp sgt i64 %3725, 0
  br i1 %3726, label %3727, label %3758

3727:                                             ; preds = %3724
  %3728 = getelementptr inbounds nuw i8, ptr %109, i64 8
  %3729 = load ptr, ptr %3728, align 8, !nonnull !ID, !noundef !ID
  %3730 = mul nuw i64 %3725, 3
  %3731 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3732 = call i64 @llvm.umin.i64(i64 %3730, i64 9223372036854775807)
  %3733 = call i64 @llvm.ssub.sat.i64(i64 %3731, i64 %3732)
  store i64 %3733, ptr %202, align 8, !noalias !ID
  %3734 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %3735 = load i64, ptr %3734, align 8, !noalias !ID, !noundef !ID
  %3736 = icmp slt i64 %3733, %3735
  br i1 %3736, label %3737, label %.preheader2545

3737:                                             ; preds = %3727
  store i64 %3733, ptr %3734, align 8, !noalias !ID
  br label %.preheader2545

.preheader2545:                                   ; preds = %3737, %3727
  br label %3738

3738:                                             ; preds = %.preheader2545, %3741
  %3739 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3740 = icmp slt i64 %3739, 0
  br i1 %3740, label %3741, label %__rustc::__rust_dealloc (.exit309)

3741:                                             ; preds = %3738
  %3742 = add nsw i64 %3739, 1
  %3743 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3739, i64 %3742 acq_rel acquire, align 8, !noalias !ID
  %3744 = extractvalue { i64, i1 } %3743, 1
  br i1 %3744, label %3745, label %3738

3745:                                             ; preds = %3741
  %3746 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3732 monotonic, align 8, !noalias !ID
  %3747 = call i64 @llvm.ssub.sat.i64(i64 %3746, i64 %3732)
  %3748 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3749

3749:                                             ; preds = %3752, %3745
  %3750 = phi i64 [ %3748, %3745 ], [ %3755, %3752 ]
  %3751 = icmp slt i64 %3747, %3750
  br i1 %3751, label %3752, label %3756

3752:                                             ; preds = %3749
  %3753 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3750, i64 %3747 monotonic monotonic, align 8, !noalias !ID
  %3754 = extractvalue { i64, i1 } %3753, 1
  %3755 = extractvalue { i64, i1 } %3753, 0
  br i1 %3754, label %3756, label %3749

3756:                                             ; preds = %3752, %3749
  %3757 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit309)

__rustc::__rust_dealloc (.exit309): ; preds = %3738, %3756
  call void @free(ptr noundef nonnull %3729) #ATTR, !noalias !ID
  br label %3758

3758:                                             ; preds = %__rustc::__rust_dealloc (.exit309), %3724
  %3759 = getelementptr inbounds nuw i8, ptr %109, i64 96
  %3760 = load ptr, ptr %3759, align 8, !noundef !ID
  %3761 = icmp eq ptr %3760, null
  br i1 %3761, label %3766, label %3762

3762:                                             ; preds = %3758
  %3763 = atomicrmw sub ptr %3760, i64 1 release, align 8, !noalias !ID
  %3764 = icmp eq i64 %3763, 1
  br i1 %3764, label %3765, label %3766

3765:                                             ; preds = %3762
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %3759) #ATTR
          to label %3766 unwind label %299

3766:                                             ; preds = %3765, %3762, %3758
  %3767 = trunc nuw i8 %3339 to i1
  br i1 %3767, label %3768, label %294

3768:                                             ; preds = %3766, %390, %387, %383
  %3769 = atomicrmw sub ptr %193, i64 1 release, align 8, !noalias !ID
  %3770 = icmp eq i64 %3769, 1
  br i1 %3770, label %3771, label %294

3771:                                             ; preds = %3768
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %157) #ATTR
  br label %294

.loopexit385:                                     ; preds = %600, %611, %523
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %148, ptr noundef nonnull align 8 dereferenceable(40) %102, i64 40, i1 false), !noalias !ID
  call void @llvm.lifetime.end.p0(ptr nonnull %102), !noalias !ID
  %3772 = load i64, ptr %488, align 8, !noundef !ID
  call void @llvm.lifetime.start.p0(ptr nonnull %105)
; invoke <hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::rustc_entry
  invoke fastcc void @<hashbrown::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>::rustc_entry(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %105, ptr noalias nofree noundef align 8 dereferenceable(32) %149, ptr noalias nofree noundef align 8 captures(address) dereferenceable(40) %148)
          to label %3773 unwind label %494

3773:                                             ; preds = %.loopexit385
  %3774 = load i64, ptr %105, align 8, !noundef !ID
  %3775 = icmp eq i64 %3774, 0
  %3776 = load ptr, ptr %489, align 8
  br i1 %3775, label %3816, label %3777

3777:                                             ; preds = %3773
  %3778 = load ptr, ptr %490, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %104, ptr noundef nonnull align 8 dereferenceable(16) %491, i64 16, i1 false)
  %3779 = load ptr, ptr %492, align 8
  %3780 = load i64, ptr %493, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %105)
  %3781 = call noundef dereferenceable_or_null(8) ptr @malloc(i64 noundef range(i64 1, 0) 8) #ATTR
  %3782 = icmp eq ptr %3781, null
  br i1 %3782, label %__rustc::__rust_alloc (.exit310.thread), label %3783

3783:                                             ; preds = %3777
  %3784 = load i64, ptr %196, align 8, !noundef !ID
  %3785 = call i64 @llvm.uadd.sat.i64(i64 %3784, i64 1)
  store i64 %3785, ptr %196, align 8
  %3786 = load i64, ptr %199, align 8, !noundef !ID
  %3787 = call i64 @llvm.uadd.sat.i64(i64 %3786, i64 8)
  store i64 %3787, ptr %199, align 8
  %3788 = load i64, ptr %202, align 8, !noundef !ID
  %3789 = call i64 @llvm.sadd.sat.i64(i64 %3788, i64 8)
  store i64 %3789, ptr %202, align 8
  %3790 = load i64, ptr %205, align 8, !noundef !ID
  %3791 = icmp sgt i64 %3789, %3790
  br i1 %3791, label %3792, label %.preheader2969

3792:                                             ; preds = %3783
  store i64 %3789, ptr %205, align 8
  br label %.preheader2969

.preheader2969:                                   ; preds = %3792, %3783
  br label %3793

3793:                                             ; preds = %.preheader2969, %3796
  %3794 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8
  %3795 = icmp slt i64 %3794, 0
  br i1 %3795, label %3796, label %__rustc::__rust_alloc (.exit310)

3796:                                             ; preds = %3793
  %3797 = add nsw i64 %3794, 1
  %3798 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3794, i64 %3797 acq_rel acquire, align 8
  %3799 = extractvalue { i64, i1 } %3798, 1
  br i1 %3799, label %3800, label %3793

3800:                                             ; preds = %3796
  %3801 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8
  %3802 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 8 monotonic, align 8
  %3803 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 8 monotonic, align 8
  %3804 = call i64 @llvm.sadd.sat.i64(i64 %3803, i64 8)
  %3805 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8
  br label %3806

3806:                                             ; preds = %3809, %3800
  %3807 = phi i64 [ %3805, %3800 ], [ %3812, %3809 ]
  %3808 = icmp sgt i64 %3804, %3807
  br i1 %3808, label %3809, label %3813

3809:                                             ; preds = %3806
  %3810 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %3807, i64 %3804 monotonic monotonic, align 8
  %3811 = extractvalue { i64, i1 } %3810, 1
  %3812 = extractvalue { i64, i1 } %3810, 0
  br i1 %3811, label %3813, label %3806

3813:                                             ; preds = %3809, %3806
  %3814 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8
  br label %__rustc::__rust_alloc (.exit310)

__rustc::__rust_alloc (.exit310.thread): ; preds = %3777
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 8) #ATTR
          to label %3815 unwind label %3895

3815:                                             ; preds = %__rustc::__rust_alloc (.exit310.thread)
  unreachable

3816:                                             ; preds = %3773
  call void @llvm.lifetime.end.p0(ptr nonnull %105)
  %3817 = getelementptr inbounds i8, ptr %3776, i64 -24
  %3818 = getelementptr inbounds i8, ptr %3776, i64 -8
  %3819 = load i64, ptr %3818, align 8, !alias.scope !ID, !noundef !ID
  %3820 = load i64, ptr %3817, align 8, !range !ID, !alias.scope !ID, !noundef !ID
  %3821 = icmp eq i64 %3819, %3820
  br i1 %3821, label %3822, label %3823

3822:                                             ; preds = %3816
; invoke <alloc::raw_vec::RawVec<usize>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<usize>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %3817) #ATTR
          to label %3823 unwind label %494

3823:                                             ; preds = %3822, %3816
  %3824 = getelementptr inbounds i8, ptr %3776, i64 -16
  %3825 = load ptr, ptr %3824, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %3826 = getelementptr inbounds nuw [8 x i8], ptr %3825, i64 %3819
  store i64 %500, ptr %3826, align 8
  %3827 = add i64 %3819, 1
  store i64 %3827, ptr %3818, align 8, !alias.scope !ID
  br label %3828

3828:                                             ; preds = %3867, %3823
  call void @llvm.lifetime.end.p0(ptr nonnull %148)
  %3829 = icmp eq ptr %501, %481
  br i1 %3829, label %630, label %498

__rustc::__rust_alloc (.exit310): ; preds = %3793, %3813
  store i64 %500, ptr %3781, align 8
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %3779) ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3830 = load ptr, ptr %3779, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %3831 = getelementptr inbounds nuw i8, ptr %3779, i64 8
  %3832 = load i64, ptr %3831, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3833 = and i64 %3832, %3780
  %3834 = getelementptr inbounds nuw i8, ptr %3830, i64 %3833
  %3835 = load <16 x i8>, ptr %3834, align 1, !noalias !ID
  %3836 = icmp slt <16 x i8> %3835, zeroinitializer
  %3837 = bitcast <16 x i1> %3836 to i16
  %3838 = icmp eq i16 %3837, 0
  br i1 %3838, label %.preheader383, label %.loopexit384, !prof !ID

.loopexit384:                                     ; preds = %.preheader383, %__rustc::__rust_alloc (.exit310)
  %3839 = phi i64 [ %3833, %__rustc::__rust_alloc (.exit310) ], [ %3861, %.preheader383 ]
  %3840 = phi i16 [ %3837, %__rustc::__rust_alloc (.exit310) ], [ %3865, %.preheader383 ]
  %3841 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %3840, i1 true)
  %3842 = zext nneg i16 %3841 to i64
  %3843 = add i64 %3839, %3842
  %3844 = and i64 %3843, %3832
  %3845 = getelementptr inbounds nuw i8, ptr %3830, i64 %3844
  %3846 = load i8, ptr %3845, align 1, !noalias !ID, !noundef !ID
  %3847 = icmp sgt i8 %3846, -1
  br i1 %3847, label %3848, label %3867, !prof !ID

3848:                                             ; preds = %.loopexit384
  %3849 = load <16 x i8>, ptr %3830, align 16, !noalias !ID
  %3850 = icmp slt <16 x i8> %3849, zeroinitializer
  %3851 = bitcast <16 x i1> %3850 to i16
  %3852 = icmp ne i16 %3851, 0
  %3853 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %3851, i1 true)
  %3854 = zext nneg i16 %3853 to i64
  call void @llvm.assume(i1 %3852)
  %3855 = getelementptr inbounds nuw i8, ptr %3830, i64 %3854
  %3856 = load i8, ptr %3855, align 1, !noalias !ID
  br label %3867

.preheader383:                                    ; preds = %__rustc::__rust_alloc (.exit310), %.preheader383
  %3857 = phi i64 [ %3861, %.preheader383 ], [ %3833, %__rustc::__rust_alloc (.exit310) ]
  %3858 = phi i64 [ %3859, %.preheader383 ], [ 0, %__rustc::__rust_alloc (.exit310) ]
  %3859 = add i64 %3858, 16
  %3860 = add i64 %3859, %3857
  %3861 = and i64 %3860, %3832
  %3862 = getelementptr inbounds nuw i8, ptr %3830, i64 %3861
  %3863 = load <16 x i8>, ptr %3862, align 1, !noalias !ID
  %3864 = icmp slt <16 x i8> %3863, zeroinitializer
  %3865 = bitcast <16 x i1> %3864 to i16
  %3866 = icmp eq i16 %3865, 0
  br i1 %3866, label %.preheader383, label %.loopexit384, !prof !ID

3867:                                             ; preds = %3848, %.loopexit384
  %3868 = phi i8 [ %3856, %3848 ], [ %3846, %.loopexit384 ]
  %3869 = phi i64 [ %3854, %3848 ], [ %3844, %.loopexit384 ]
  %3870 = getelementptr inbounds nuw i8, ptr %3830, i64 %3869
  %3871 = lshr i64 %3780, 57
  %3872 = trunc nuw nsw i64 %3871 to i8
  %3873 = add i64 %3869, -16
  %3874 = and i64 %3873, %3832
  store i8 %3872, ptr %3870, align 1, !noalias !ID
  %3875 = getelementptr i8, ptr %3830, i64 %3874
  %3876 = getelementptr i8, ptr %3875, i64 16
  store i8 %3872, ptr %3876, align 1, !noalias !ID
  %3877 = sub nsw i64 0, %3869
  %3878 = getelementptr inbounds [72 x i8], ptr %3830, i64 %3877
  %3879 = and i8 %3868, 1
  %3880 = zext nneg i8 %3879 to i64
  %3881 = getelementptr inbounds nuw i8, ptr %3779, i64 16
  %3882 = load i64, ptr %3881, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3883 = sub i64 %3882, %3880
  store i64 %3883, ptr %3881, align 8, !alias.scope !ID, !noalias !ID
  %3884 = getelementptr inbounds i8, ptr %3878, i64 -72
  store i64 %3774, ptr %3884, align 8, !noalias !ID
  %3885 = getelementptr inbounds i8, ptr %3878, i64 -64
  store ptr %3776, ptr %3885, align 8, !noalias !ID
  %3886 = getelementptr inbounds i8, ptr %3878, i64 -56
  store ptr %3778, ptr %3886, align 8, !noalias !ID
  %3887 = getelementptr inbounds i8, ptr %3878, i64 -48
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %3887, ptr noundef nonnull align 8 dereferenceable(16) %104, i64 16, i1 false)
  %3888 = getelementptr inbounds i8, ptr %3878, i64 -32
  store i64 %3772, ptr %3888, align 8, !noalias !ID
  %3889 = getelementptr inbounds i8, ptr %3878, i64 -24
  store i64 1, ptr %3889, align 8, !noalias !ID
  %3890 = getelementptr inbounds i8, ptr %3878, i64 -16
  store ptr %3781, ptr %3890, align 8, !noalias !ID
  %3891 = getelementptr inbounds i8, ptr %3878, i64 -8
  store i64 1, ptr %3891, align 8, !noalias !ID
  %3892 = getelementptr inbounds nuw i8, ptr %3779, i64 24
  %3893 = load i64, ptr %3892, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3894 = add i64 %3893, 1
  store i64 %3894, ptr %3892, align 8, !alias.scope !ID, !noalias !ID
  br label %3828

3895:                                             ; preds = %__rustc::__rust_alloc (.exit310.thread)
  %3896 = landingpad { ptr, i32 }
          cleanup
  %3897 = icmp ugt i64 %3774, 5
  br i1 %3897, label %3898, label %3933

3898:                                             ; preds = %3895
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %3776) ]
  %3899 = shl i64 %3774, 3
  %3900 = add i64 %3899, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3776, i64 noundef %3900, i64 noundef range(i64 1, -9223372036854775807) 4) #ATTR, !noalias !ID
  br label %3933

3901:                                             ; preds = %3933, %865, %848, %753
  %3902 = phi { ptr, i32 } [ %3934, %3933 ], [ %868, %865 ], [ %849, %848 ], [ %744, %753 ]
  %3903 = phi i8 [ 1, %3933 ], [ %867, %865 ], [ 1, %848 ], [ 1, %753 ]
  %3904 = phi i1 [ true, %3933 ], [ %866, %865 ], [ true, %848 ], [ true, %753 ]
  br i1 %423, label %402, label %3905

3905:                                             ; preds = %3901
  %3906 = load i64, ptr %202, align 8, !noundef !ID
  %3907 = call i64 @llvm.umin.i64(i64 %422, i64 9223372036854775807)
  %3908 = call i64 @llvm.ssub.sat.i64(i64 %3906, i64 %3907)
  store i64 %3908, ptr %202, align 8
  %3909 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %3910 = load i64, ptr %3909, align 8, !noundef !ID
  %3911 = icmp slt i64 %3908, %3910
  br i1 %3911, label %3912, label %.preheader2552

3912:                                             ; preds = %3905
  store i64 %3908, ptr %3909, align 8
  br label %.preheader2552

.preheader2552:                                   ; preds = %3912, %3905
  br label %3913

3913:                                             ; preds = %.preheader2552, %3916
  %3914 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8
  %3915 = icmp slt i64 %3914, 0
  br i1 %3915, label %3916, label %__rustc::__rust_dealloc (.exit311)

3916:                                             ; preds = %3913
  %3917 = add nsw i64 %3914, 1
  %3918 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3914, i64 %3917 acq_rel acquire, align 8
  %3919 = extractvalue { i64, i1 } %3918, 1
  br i1 %3919, label %3920, label %3913

3920:                                             ; preds = %3916
  %3921 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3907 monotonic, align 8
  %3922 = call i64 @llvm.ssub.sat.i64(i64 %3921, i64 %3907)
  %3923 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %3924

3924:                                             ; preds = %3927, %3920
  %3925 = phi i64 [ %3923, %3920 ], [ %3930, %3927 ]
  %3926 = icmp slt i64 %3922, %3925
  br i1 %3926, label %3927, label %3931

3927:                                             ; preds = %3924
  %3928 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3925, i64 %3922 monotonic monotonic, align 8
  %3929 = extractvalue { i64, i1 } %3928, 1
  %3930 = extractvalue { i64, i1 } %3928, 0
  br i1 %3929, label %3931, label %3924

3931:                                             ; preds = %3927, %3924
  %3932 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit311)

__rustc::__rust_dealloc (.exit311): ; preds = %3913, %3931
  call void @free(ptr noundef nonnull %475) #ATTR
  br label %402

3933:                                             ; preds = %3898, %3895, %626, %622, %496, %494
  %3934 = phi { ptr, i32 } [ %623, %626 ], [ %3896, %3898 ], [ %623, %622 ], [ %3896, %3895 ], [ %495, %494 ], [ %497, %496 ]
; call core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>
  call fastcc void @core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, (usize, alloc::vec::Vec<usize>), purrdf_hash::fixed::FixedState>>(ptr noalias nofree noundef align 8 dereferenceable(32) %149) #ATTR
  br label %3901

3935:                                             ; preds = %393
  call void @llvm.lifetime.end.p0(ptr nonnull %151)
  call void @llvm.lifetime.end.p0(ptr nonnull %152)
  call void @llvm.lifetime.start.p0(ptr nonnull %156)
  call void @llvm.lifetime.start.p0(ptr nonnull %155)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %155, ptr noundef nonnull align 8 dereferenceable(104) %109, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %154)
  %3936 = getelementptr inbounds nuw i8, ptr %154, i64 24
  store ptr %193, ptr %3936, align 8, !alias.scope !ID
  store i64 0, ptr %154, align 8, !alias.scope !ID
  %3937 = getelementptr inbounds nuw i8, ptr %154, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %3937, align 8, !alias.scope !ID
  %3938 = getelementptr inbounds nuw i8, ptr %154, i64 16
  store i64 0, ptr %3938, align 8, !alias.scope !ID
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3939 = load i64, ptr %155, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3940 = icmp eq i64 %3939, -1
  br i1 %3940, label %3942, label %3941

3941:                                             ; preds = %3935
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %156, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %154, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %109)
  br label %3944

3942:                                             ; preds = %3935
  %3943 = getelementptr inbounds nuw i8, ptr %156, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %3943, ptr noundef nonnull readonly align 8 dereferenceable(32) %154, i64 32, i1 false), !alias.scope !ID, !noalias !ID
  store i64 -1, ptr %156, align 8, !alias.scope !ID, !noalias !ID
  br label %3944

3944:                                             ; preds = %3942, %3941
  %3945 = getelementptr inbounds nuw i8, ptr %155, i64 72
  %3946 = load i64, ptr %3945, align 8, !range !ID, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3947 = icmp ugt i64 %3946, 5
  br i1 %3947, label %3948, label %3981

3948:                                             ; preds = %3944
  %3949 = getelementptr inbounds nuw i8, ptr %155, i64 80
  %3950 = load ptr, ptr %3949, align 8, !alias.scope !ID, !noalias !ID, !nonnull !ID, !noundef !ID
  %3951 = mul i64 %3946, 3
  %3952 = add i64 %3951, -3
  %3953 = load i64, ptr %202, align 8, !noalias !ID, !noundef !ID
  %3954 = call i64 @llvm.umin.i64(i64 %3952, i64 9223372036854775807)
  %3955 = call i64 @llvm.ssub.sat.i64(i64 %3953, i64 %3954)
  store i64 %3955, ptr %202, align 8, !noalias !ID
  %3956 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.ID))
  %3957 = load i64, ptr %3956, align 8, !noalias !ID, !noundef !ID
  %3958 = icmp slt i64 %3955, %3957
  br i1 %3958, label %3959, label %.preheader2544

3959:                                             ; preds = %3948
  store i64 %3955, ptr %3956, align 8, !noalias !ID
  br label %.preheader2544

.preheader2544:                                   ; preds = %3959, %3948
  br label %3960

3960:                                             ; preds = %.preheader2544, %3963
  %3961 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID) acquire, align 8, !noalias !ID
  %3962 = icmp slt i64 %3961, 0
  br i1 %3962, label %3963, label %__rustc::__rust_dealloc (.exit312)

3963:                                             ; preds = %3960
  %3964 = add nsw i64 %3961, 1
  %3965 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 %3961, i64 %3964 acq_rel acquire, align 8, !noalias !ID
  %3966 = extractvalue { i64, i1 } %3965, 1
  br i1 %3966, label %3967, label %3960

3967:                                             ; preds = %3963
  %3968 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %3954 monotonic, align 8, !noalias !ID
  %3969 = call i64 @llvm.ssub.sat.i64(i64 %3968, i64 %3954)
  %3970 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !ID
  br label %3971

3971:                                             ; preds = %3974, %3967
  %3972 = phi i64 [ %3970, %3967 ], [ %3977, %3974 ]
  %3973 = icmp slt i64 %3969, %3972
  br i1 %3973, label %3974, label %3978

3974:                                             ; preds = %3971
  %3975 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %3972, i64 %3969 monotonic monotonic, align 8, !noalias !ID
  %3976 = extractvalue { i64, i1 } %3975, 1
  %3977 = extractvalue { i64, i1 } %3975, 0
  br i1 %3976, label %3978, label %3971

3978:                                             ; preds = %3974, %3971
  %3979 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.ID), i64 1 release, align 8, !noalias !ID
  br label %__rustc::__rust_dealloc (.exit312)

__rustc::__rust_dealloc (.exit312): ; preds = %3960, %3978
  %3980 = icmp ne i64 %3952, 0
  call void @llvm.assume(i1 %3980), !noalias !ID
  call void @free(ptr noundef nonnull %3950) #ATTR, !noalias !ID
  br label %3981

3981:                                             ; preds = %__rustc::__rust_dealloc (.exit312), %3944
  %3982 = getelementptr inbounds nuw i8, ptr %155, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3983 = load ptr, ptr %3982, align 8, !alias.scope !ID, !noalias !ID, !noundef !ID
  %3984 = icmp eq ptr %3983, null
  br i1 %3984, label %3989, label %3985

3985:                                             ; preds = %3981
  %3986 = atomicrmw sub ptr %3983, i64 1 release, align 8, !noalias !ID
  %3987 = icmp eq i64 %3986, 1
  br i1 %3987, label %3988, label %3989

3988:                                             ; preds = %3985
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %3982) #ATTR
  br label %3989

3989:                                             ; preds = %3988, %3985, %3981
  call void @llvm.lifetime.end.p0(ptr nonnull %154)
  call void @llvm.lifetime.end.p0(ptr nonnull %155)
  %3990 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %3990, ptr noundef nonnull align 8 dereferenceable(96) %156, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %156)
  call void @llvm.lifetime.end.p0(ptr nonnull %157)
  call void @llvm.lifetime.end.p0(ptr nonnull %158)
  br label %3526

3991:                                             ; preds = %304, %303
  %3992 = phi { ptr, i32 } [ %306, %304 ], [ %3520, %303 ]
  %3993 = phi i8 [ %305, %304 ], [ %3519, %303 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %3994 = getelementptr inbounds nuw i8, ptr %109, i64 72
  %3995 = load i64, ptr %3994, align 8, !range !ID, !alias.scope !ID, !noundef !ID
  %3996 = icmp ugt i64 %3995, 5
  br i1 %3996, label %3997, label %4002

3997:                                             ; preds = %3991
  %3998 = getelementptr inbounds nuw i8, ptr %109, i64 80
  %3999 = load ptr, ptr %3998, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %4000 = mul i64 %3995, 3
  %4001 = add i64 %4000, -3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %3999, i64 noundef %4001, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %4002

4002:                                             ; preds = %3997, %3991
  %4003 = load i64, ptr %109, align 8, !range !ID, !alias.scope !ID, !noundef !ID
  %4004 = icmp sgt i64 %4003, 0
  br i1 %4004, label %4005, label %4009

4005:                                             ; preds = %4002
  %4006 = getelementptr inbounds nuw i8, ptr %109, i64 8
  %4007 = load ptr, ptr %4006, align 8, !alias.scope !ID, !nonnull !ID, !noundef !ID
  %4008 = mul nuw i64 %4003, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %4007, i64 noundef %4008, i64 noundef range(i64 1, -9223372036854775807) 1) #ATTR, !noalias !ID
  br label %4009

4009:                                             ; preds = %4005, %4002
  %4010 = getelementptr inbounds nuw i8, ptr %109, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !ID)
  %4011 = load ptr, ptr %4010, align 8, !alias.scope !ID, !noundef !ID
  %4012 = icmp eq ptr %4011, null
  br i1 %4012, label %295, label %4013

4013:                                             ; preds = %4009
  %4014 = atomicrmw sub ptr %4011, i64 1 release, align 8, !noalias !ID
  %4015 = icmp eq i64 %4014, 1
  br i1 %4015, label %4016, label %295

4016:                                             ; preds = %4013
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %4010) #ATTR
          to label %295 unwind label %1290

4017:                                             ; preds = %295
  %4018 = atomicrmw sub ptr %193, i64 1 release, align 8, !noalias !ID
  %4019 = icmp eq i64 %4018, 1
  br i1 %4019, label %4020, label %181

4020:                                             ; preds = %4017
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %157) #ATTR
          to label %181 unwind label %1290

4021:                                             ; preds = %243
  %4022 = landingpad { ptr, i32 }
          cleanup
  br label %4025

4023:                                             ; preds = %__rustc::__rust_alloc (.exit288.thread)
  %4024 = landingpad { ptr, i32 }
          cleanup
  br label %4025

4025:                                             ; preds = %4023, %4021
  %4026 = phi { ptr, i32 } [ %4022, %4021 ], [ %4024, %4023 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.ID)(ptr noalias nofree noundef align 8 dereferenceable(56) %159) #ATTR
          to label %181 unwind label %1290
}
