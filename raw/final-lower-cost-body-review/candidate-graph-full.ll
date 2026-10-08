define void @purrdf_sparql_eval::modifier::eval_graph_with::<purrdf_core::ir::dataset::RdfDataset, ()>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %2, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef align 16 dereferenceable(1248) %4) unnamed_addr #8 personality ptr @rust_eh_personality !guid !38892 {
  %6 = alloca [16 x i8], align 8
  %7 = alloca [8 x i8], align 8
  %8 = alloca [16 x i8], align 8
  %9 = alloca [24 x i8], align 8
  %10 = alloca [16 x i8], align 8
  %11 = alloca [72 x i8], align 8
  %12 = alloca [8 x i8], align 8
  %13 = alloca [56 x i8], align 8
  %14 = alloca [72 x i8], align 8
  %15 = alloca [56 x i8], align 8
  %16 = alloca [8 x i8], align 8
  %17 = alloca [72 x i8], align 8
  %18 = alloca [16 x i8], align 4
  %19 = alloca [88 x i8], align 8
  %20 = alloca [24 x i8], align 8
  %21 = alloca [40 x i8], align 8
  %22 = alloca [32 x i8], align 8
  %23 = alloca [32 x i8], align 8
  %24 = alloca [40 x i8], align 8
  %25 = alloca [32 x i8], align 8
  %26 = alloca [56 x i8], align 8
  %27 = alloca [72 x i8], align 8
  %28 = alloca [112 x i8], align 16
  %29 = alloca [72 x i8], align 8
  %30 = alloca [72 x i8], align 8
  %31 = alloca [32 x i8], align 8
  %32 = alloca [104 x i8], align 8
  %33 = alloca [96 x i8], align 8
  %34 = alloca [56 x i8], align 8
  %35 = alloca [104 x i8], align 8
  %36 = alloca [32 x i8], align 8
  %37 = alloca [32 x i8], align 8
  %38 = alloca [104 x i8], align 8
  %39 = alloca [96 x i8], align 8
  %40 = alloca [8 x i8], align 8
  %41 = alloca [56 x i8], align 8
  %42 = alloca [56 x i8], align 8
  %43 = alloca [56 x i8], align 8
  %44 = alloca [96 x i8], align 8
  %45 = alloca [32 x i8], align 8
  %46 = alloca [112 x i8], align 16
  %47 = alloca [32 x i8], align 8
  %48 = alloca [24 x i8], align 8
  %49 = alloca [56 x i8], align 8
  %50 = alloca [112 x i8], align 16
  %51 = alloca [32 x i8], align 8
  %52 = alloca [104 x i8], align 8
  %53 = alloca [96 x i8], align 8
  %54 = alloca [104 x i8], align 8
  %55 = alloca [96 x i8], align 8
  %56 = alloca [32 x i8], align 8
  %57 = alloca [32 x i8], align 8
  %58 = alloca [104 x i8], align 8
  %59 = alloca [96 x i8], align 8
  %60 = alloca [80 x i8], align 8
  %61 = load i64, ptr %2, align 8, !range !1739, !noundef !1740
  %62 = trunc nuw i64 %61 to i1
  %63 = getelementptr inbounds nuw i8, ptr %2, i64 8
  br i1 %62, label %70, label %1461

64:                                               ; preds = %1450, %1351, %245
  %65 = phi i8 [ %1025, %245 ], [ %1353, %1450 ], [ %1353, %1351 ]
  %66 = phi { ptr, i32 } [ %246, %245 ], [ %1355, %1450 ], [ %1355, %1351 ]
  %67 = trunc nuw i8 %65 to i1
  br i1 %67, label %1458, label %1456

68:                                               ; preds = %__rustc::__rust_alloc (.exit.thread)
  %69 = landingpad { ptr, i32 }
          cleanup
  br label %1458

70:                                               ; preds = %5
  tail call void @llvm.experimental.noalias.scope.decl(metadata !38893)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !38896)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !38898)
  call void @llvm.lifetime.start.p0(ptr nonnull %47)
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %38, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1), !inline_history !38900
  %71 = getelementptr inbounds nuw i8, ptr %4, i64 664
  %72 = load ptr, ptr %71, align 8, !alias.scope !38898, !noalias !38901, !nonnull !1740, !align !1836, !noundef !1740
  %73 = getelementptr i8, ptr %72, i64 152
  %74 = load ptr, ptr %73, align 8, !noalias !38904, !nonnull !1740, !noundef !1740
  %75 = getelementptr i8, ptr %72, i64 160
  %76 = load i64, ptr %75, align 8, !noalias !38904, !noundef !1740
  %77 = shl nuw nsw i64 %76, 2
  %78 = getelementptr inbounds nuw i8, ptr %74, i64 %77
  %79 = getelementptr inbounds nuw i8, ptr %4, i64 1144
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !38905
  %80 = icmp eq i64 %76, 0
  br i1 %80, label %.loopexit112, label %81

81:                                               ; preds = %70
  %82 = load i64, ptr %79, align 8, !range !1739, !noalias !38912, !noundef !1740
  %83 = trunc nuw i64 %82 to i1
  %84 = getelementptr inbounds nuw i8, ptr %4, i64 1160
  br i1 %83, label %85, label %122

85:                                               ; preds = %81
  %86 = getelementptr inbounds nuw i8, ptr %4, i64 1152
  %87 = load ptr, ptr %86, align 16, !noalias !38912, !noundef !1740
  %88 = icmp eq ptr %87, null
  br i1 %88, label %.loopexit112, label %89

89:                                               ; preds = %85
  %90 = load i64, ptr %84, align 8, !noalias !38912
  br label %91

91:                                               ; preds = %120, %89
  %92 = phi ptr [ %74, %89 ], [ %93, %120 ]
  %93 = getelementptr inbounds nuw i8, ptr %92, i64 4
  %94 = load i32, ptr %92, align 4, !range !3837, !noalias !38912, !noundef !1740
  br label %95

95:                                               ; preds = %114, %91
  %96 = phi i64 [ %119, %114 ], [ %90, %91 ]
  %97 = phi ptr [ %118, %114 ], [ %87, %91 ]
  %98 = getelementptr inbounds nuw i8, ptr %97, i64 8
  %99 = getelementptr inbounds nuw i8, ptr %97, i64 54
  %100 = load i16, ptr %99, align 2, !noalias !38921, !noundef !1740
  %101 = zext i16 %100 to i64
  %.idx1076 = shl nuw nsw i64 %101, 2
  %102 = getelementptr inbounds nuw i8, ptr %98, i64 %.idx1076
  %103 = icmp eq i16 %100, 0
  br i1 %103, label %._crit_edge1066, label %.lr.ph1065

104:                                              ; preds = %.lr.ph1065
  %105 = getelementptr inbounds nuw i8, ptr %108, i64 4
  %106 = add nuw nsw i64 %109, 1
  %107 = icmp eq ptr %105, %102
  br i1 %107, label %._crit_edge1066, label %.lr.ph1065

.lr.ph1065:                                       ; preds = %95, %104
  %108 = phi ptr [ %105, %104 ], [ %98, %95 ]
  %109 = phi i64 [ %106, %104 ], [ 0, %95 ]
  %110 = load i32, ptr %108, align 4, !range !3837, !noalias !38921, !noundef !1740
  %111 = tail call noundef range(i8 -1, 2) i8 @llvm.ucmp.i8.i32(i32 range(i32 1, 0) %94, i32 %110)
  switch i8 %111, label %125 [
    i8 -1, label %._crit_edge1066
    i8 0, label %.loopexit1263
    i8 1, label %104
  ]

._crit_edge1066:                                  ; preds = %104, %.lr.ph1065, %95
  %112 = phi i64 [ %101, %95 ], [ %101, %104 ], [ %109, %.lr.ph1065 ]
  %113 = icmp eq i64 %96, 0
  br i1 %113, label %120, label %114

114:                                              ; preds = %._crit_edge1066
  %115 = getelementptr inbounds nuw i8, ptr %97, i64 56
  %116 = icmp samesign ult i64 %112, 12
  tail call void @llvm.assume(i1 %116), !noalias !38924
  %117 = getelementptr inbounds nuw [8 x i8], ptr %115, i64 %112
  %118 = load ptr, ptr %117, align 8, !noalias !38921, !nonnull !1740, !noundef !1740
  %119 = add i64 %96, -1
  br label %95

120:                                              ; preds = %._crit_edge1066
  %121 = icmp eq ptr %93, %78
  br i1 %121, label %.loopexit112, label %91

122:                                              ; preds = %81
  %123 = getelementptr inbounds nuw i8, ptr %74, i64 4
  %124 = load i32, ptr %74, align 4, !range !3837, !noalias !38912, !noundef !1740
  br label %.loopexit1263

125:                                              ; preds = %.lr.ph1065
  unreachable

.loopexit1263:                                    ; preds = %.lr.ph1065, %122
  %126 = phi ptr [ %123, %122 ], [ %93, %.lr.ph1065 ]
  %127 = phi i32 [ %124, %122 ], [ %94, %.lr.ph1065 ]
  %128 = tail call noundef dereferenceable_or_null(16) ptr @malloc(i64 noundef range(i64 1, 0) 16) #93, !noalias !38925
  %129 = icmp eq ptr %128, null
  br i1 %129, label %__rustc::__rust_alloc (.exit.thread), label %130

130:                                              ; preds = %.loopexit1263
  %131 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %132 = load i64, ptr %131, align 8, !noalias !38925, !noundef !1740
  %133 = tail call i64 @llvm.uadd.sat.i64(i64 %132, i64 1)
  store i64 %133, ptr %131, align 8, !noalias !38925
  %134 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %135 = load i64, ptr %134, align 8, !noalias !38925, !noundef !1740
  %136 = tail call i64 @llvm.uadd.sat.i64(i64 %135, i64 16)
  store i64 %136, ptr %134, align 8, !noalias !38925
  %137 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %138 = load i64, ptr %137, align 8, !noalias !38925, !noundef !1740
  %139 = tail call i64 @llvm.sadd.sat.i64(i64 %138, i64 16)
  store i64 %139, ptr %137, align 8, !noalias !38925
  %140 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %141 = load i64, ptr %140, align 8, !noalias !38925, !noundef !1740
  %142 = icmp sgt i64 %139, %141
  br i1 %142, label %143, label %.preheader1261

143:                                              ; preds = %130
  store i64 %139, ptr %140, align 8, !noalias !38925
  br label %.preheader1261

.preheader1261:                                   ; preds = %143, %130
  br label %144

144:                                              ; preds = %.preheader1261, %147
  %145 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !38925
  %146 = icmp slt i64 %145, 0
  br i1 %146, label %147, label %__rustc::__rust_alloc (.exit)

147:                                              ; preds = %144
  %148 = add nsw i64 %145, 1
  %149 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %145, i64 %148 acq_rel acquire, align 8, !noalias !38925
  %150 = extractvalue { i64, i1 } %149, 1
  br i1 %150, label %151, label %144

151:                                              ; preds = %147
  %152 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !38925
  %153 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 16 monotonic, align 8, !noalias !38925
  %154 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 16 monotonic, align 8, !noalias !38925
  %155 = tail call i64 @llvm.sadd.sat.i64(i64 %154, i64 16)
  %156 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !38925
  br label %157

157:                                              ; preds = %160, %151
  %158 = phi i64 [ %156, %151 ], [ %163, %160 ]
  %159 = icmp sgt i64 %155, %158
  br i1 %159, label %160, label %164

160:                                              ; preds = %157
  %161 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %158, i64 %155 monotonic monotonic, align 8, !noalias !38925
  %162 = extractvalue { i64, i1 } %161, 1
  %163 = extractvalue { i64, i1 } %161, 0
  br i1 %162, label %164, label %157

164:                                              ; preds = %160, %157
  %165 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !38925
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %.loopexit1263
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 4, i64 16) #94
          to label %166 unwind label %68

166:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %144, %164
  store i32 %127, ptr %128, align 4, !noalias !38905
  store i64 4, ptr %9, align 8, !noalias !38905
  %167 = getelementptr inbounds nuw i8, ptr %9, i64 8
  store ptr %128, ptr %167, align 8, !noalias !38905
  %168 = getelementptr inbounds nuw i8, ptr %9, i64 16
  store i64 1, ptr %168, align 8, !noalias !38905
  tail call void @llvm.experimental.noalias.scope.decl(metadata !38928), !noalias !38924
  tail call void @llvm.experimental.noalias.scope.decl(metadata !38931), !noalias !38924
  %169 = icmp eq ptr %126, %78
  br i1 %169, label %.loopexit110, label %170

170:                                              ; preds = %__rustc::__rust_alloc (.exit)
  %171 = getelementptr inbounds nuw i8, ptr %4, i64 1152
  br label %172

172:                                              ; preds = %226, %170
  %173 = phi ptr [ %128, %170 ], [ %227, %226 ]
  %174 = phi i64 [ 1, %170 ], [ %229, %226 ]
  %175 = phi ptr [ %126, %170 ], [ %218, %226 ]
  %176 = load i64, ptr %79, align 8, !range !1739, !noalias !38934, !noundef !1740
  %177 = trunc nuw i64 %176 to i1
  br i1 %177, label %178, label %214

178:                                              ; preds = %172
  %179 = load ptr, ptr %171, align 16, !noalias !38934, !noundef !1740
  %180 = icmp eq ptr %179, null
  br i1 %180, label %.loopexit110, label %181

181:                                              ; preds = %178
  %182 = load i64, ptr %84, align 8, !noalias !38934
  br label %183

183:                                              ; preds = %212, %181
  %184 = phi ptr [ %175, %181 ], [ %185, %212 ]
  %185 = getelementptr inbounds nuw i8, ptr %184, i64 4
  %186 = load i32, ptr %184, align 4, !range !3837, !noalias !38934, !noundef !1740
  br label %187

187:                                              ; preds = %206, %183
  %188 = phi i64 [ %211, %206 ], [ %182, %183 ]
  %189 = phi ptr [ %210, %206 ], [ %179, %183 ]
  %190 = getelementptr inbounds nuw i8, ptr %189, i64 8
  %191 = getelementptr inbounds nuw i8, ptr %189, i64 54
  %192 = load i16, ptr %191, align 2, !noalias !38945, !noundef !1740
  %193 = zext i16 %192 to i64
  %.idx1077 = shl nuw nsw i64 %193, 2
  %194 = getelementptr inbounds nuw i8, ptr %190, i64 %.idx1077
  %195 = icmp eq i16 %192, 0
  br i1 %195, label %._crit_edge1072, label %.lr.ph1071

196:                                              ; preds = %.lr.ph1071
  %197 = getelementptr inbounds nuw i8, ptr %200, i64 4
  %198 = add nuw nsw i64 %201, 1
  %199 = icmp eq ptr %197, %194
  br i1 %199, label %._crit_edge1072, label %.lr.ph1071

.lr.ph1071:                                       ; preds = %187, %196
  %200 = phi ptr [ %197, %196 ], [ %190, %187 ]
  %201 = phi i64 [ %198, %196 ], [ 0, %187 ]
  %202 = load i32, ptr %200, align 4, !range !3837, !noalias !38945, !noundef !1740
  %203 = tail call noundef range(i8 -1, 2) i8 @llvm.ucmp.i8.i32(i32 range(i32 1, 0) %186, i32 %202)
  switch i8 %203, label %217 [
    i8 -1, label %._crit_edge1072
    i8 0, label %.loopexit1248
    i8 1, label %196
  ]

._crit_edge1072:                                  ; preds = %196, %.lr.ph1071, %187
  %204 = phi i64 [ %193, %187 ], [ %193, %196 ], [ %201, %.lr.ph1071 ]
  %205 = icmp eq i64 %188, 0
  br i1 %205, label %212, label %206

206:                                              ; preds = %._crit_edge1072
  %207 = getelementptr inbounds nuw i8, ptr %189, i64 56
  %208 = icmp samesign ult i64 %204, 12
  tail call void @llvm.assume(i1 %208), !noalias !38924
  %209 = getelementptr inbounds nuw [8 x i8], ptr %207, i64 %204
  %210 = load ptr, ptr %209, align 8, !noalias !38945, !nonnull !1740, !noundef !1740
  %211 = add i64 %188, -1
  br label %187

212:                                              ; preds = %._crit_edge1072
  %213 = icmp eq ptr %185, %78
  br i1 %213, label %.loopexit110, label %183

214:                                              ; preds = %172
  %215 = getelementptr inbounds nuw i8, ptr %175, i64 4
  %216 = load i32, ptr %175, align 4, !range !3837, !noalias !38934, !noundef !1740
  br label %.loopexit1248

217:                                              ; preds = %.lr.ph1071
  unreachable

.loopexit1248:                                    ; preds = %.lr.ph1071, %214
  %218 = phi ptr [ %215, %214 ], [ %185, %.lr.ph1071 ]
  %219 = phi i32 [ %216, %214 ], [ %186, %.lr.ph1071 ]
  %220 = icmp samesign ult i64 %174, 2305843009213693952
  tail call void @llvm.assume(i1 %220), !noalias !38924
  %221 = load i64, ptr %9, align 8, !range !1835, !alias.scope !38948, !noalias !38949, !noundef !1740
  %222 = icmp eq i64 %174, %221
  br i1 %222, label %223, label %226

223:                                              ; preds = %.loopexit1248
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %174, i64 noundef 1, i64 noundef 4, i64 noundef 4)
          to label %224 unwind label %231, !noalias !38905

224:                                              ; preds = %223
  %225 = load ptr, ptr %167, align 8, !alias.scope !38948, !noalias !38949
  br label %226

226:                                              ; preds = %224, %.loopexit1248
  %227 = phi ptr [ %225, %224 ], [ %173, %.loopexit1248 ]
  %228 = getelementptr inbounds nuw [4 x i8], ptr %227, i64 %174
  store i32 %219, ptr %228, align 4, !noalias !38950
  %229 = add nuw nsw i64 %174, 1
  store i64 %229, ptr %168, align 8, !alias.scope !38948, !noalias !38949
  %230 = icmp eq ptr %218, %78
  br i1 %230, label %.loopexit110, label %172

231:                                              ; preds = %223
  %232 = landingpad { ptr, i32 }
          cleanup
  %233 = load i64, ptr %9, align 8, !noalias !38905
  %234 = icmp eq i64 %233, 0
  br i1 %234, label %1458, label %235

235:                                              ; preds = %231
  %236 = load ptr, ptr %167, align 8, !noalias !38905, !nonnull !1740, !noundef !1740
  %237 = shl nuw i64 %233, 2
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %236, i64 noundef %237, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !38905
  br label %1458

.loopexit110:                                     ; preds = %226, %178, %212, %__rustc::__rust_alloc (.exit)
  %238 = phi i64 [ %174, %212 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %174, %178 ], [ %229, %226 ]
  %239 = load i64, ptr %9, align 8, !noalias !38951
  %240 = load ptr, ptr %167, align 8, !noalias !38951
  br label %.loopexit112

.loopexit112:                                     ; preds = %120, %.loopexit110, %85, %70
  %241 = phi i64 [ %238, %.loopexit110 ], [ 0, %70 ], [ 0, %85 ], [ 0, %120 ]
  %242 = phi ptr [ %240, %.loopexit110 ], [ inttoptr (i64 4 to ptr), %70 ], [ inttoptr (i64 4 to ptr), %85 ], [ inttoptr (i64 4 to ptr), %120 ]
  %243 = phi i64 [ %239, %.loopexit110 ], [ 0, %70 ], [ 0, %85 ], [ 0, %120 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !38905
; invoke purrdf_sparql_eval::modifier::yields_nothing_without_rows_in_the_active_graph
  %244 = invoke noundef zeroext i1 @purrdf_sparql_eval::modifier::yields_nothing_without_rows_in_the_active_graph(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3)
          to label %247 unwind label %1451, !noalias !38924, !inline_history !38900

245:                                              ; preds = %1363
  %246 = landingpad { ptr, i32 }
          cleanup
  br label %64

247:                                              ; preds = %.loopexit112
  %248 = getelementptr inbounds nuw i8, ptr %4, i64 784
  %249 = load i32, ptr %248, align 16, !range !1785, !alias.scope !38898, !noalias !38901, !noundef !1740
  %250 = getelementptr inbounds nuw i8, ptr %4, i64 788
  %251 = load i32, ptr %250, align 4, !alias.scope !38898, !noalias !38901
  call void @llvm.lifetime.start.p0(ptr nonnull %49), !noalias !38952
  store i64 -1, ptr %49, align 8, !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %48), !noalias !38952
  store i64 0, ptr %48, align 8, !noalias !38952
  %252 = getelementptr inbounds nuw i8, ptr %48, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %252, align 8, !noalias !38952
  %253 = getelementptr inbounds nuw i8, ptr %48, i64 16
  store i64 0, ptr %253, align 8, !noalias !38952
  %254 = icmp ult i64 %241, 2305843009213693952
  tail call void @llvm.assume(i1 %254)
  %255 = shl nuw nsw i64 %241, 2
  %256 = getelementptr inbounds nuw i8, ptr %242, i64 %255
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %242) ]
  %257 = icmp eq i64 %241, 0
  br i1 %257, label %.loopexit108, label %258

258:                                              ; preds = %247
  %259 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %260 = getelementptr inbounds nuw i8, ptr %17, i64 4
  %261 = getelementptr inbounds nuw i8, ptr %46, i64 8
  %262 = getelementptr inbounds nuw i8, ptr %46, i64 16
  %263 = getelementptr inbounds nuw i8, ptr %47, i64 24
  %264 = load ptr, ptr %63, align 8, !nonnull !1740
  %265 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %266 = load i64, ptr %265, align 8
  %267 = getelementptr inbounds nuw i8, ptr %26, i64 16
  %268 = getelementptr inbounds nuw i8, ptr %47, i64 8
  %269 = getelementptr inbounds nuw i8, ptr %47, i64 16
  %270 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %271 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %272 = getelementptr inbounds nuw i8, ptr %23, i64 24
  %273 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %274 = getelementptr inbounds nuw i8, ptr %21, i64 16
  %275 = getelementptr inbounds nuw i8, ptr %10, i64 8
  %276 = getelementptr inbounds nuw i8, ptr %25, i64 8
  %277 = getelementptr inbounds nuw i8, ptr %25, i64 16
  %278 = getelementptr inbounds nuw i8, ptr %25, i64 24
  %279 = getelementptr inbounds nuw i8, ptr %24, i64 8
  %280 = getelementptr inbounds nuw i8, ptr %24, i64 16
  %281 = getelementptr inbounds nuw i8, ptr %46, i64 40
  %282 = getelementptr inbounds nuw i8, ptr %46, i64 48
  %283 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %284 = getelementptr inbounds nuw i8, ptr %17, i64 16
  %285 = getelementptr inbounds nuw i8, ptr %17, i64 24
  %286 = getelementptr inbounds nuw i8, ptr %17, i64 48
  %287 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %288 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %289

289:                                              ; preds = %.loopexit105, %258
  %290 = phi ptr [ inttoptr (i64 8 to ptr), %258 ], [ %301, %.loopexit105 ]
  %291 = phi i64 [ 0, %258 ], [ %302, %.loopexit105 ]
  %292 = phi i1 [ false, %258 ], [ true, %.loopexit105 ]
  %293 = phi ptr [ %242, %258 ], [ %304, %.loopexit105 ]
  br label %300

294:                                              ; preds = %.loopexit106, %.loopexit.split-lp, %1350, %817, %813, %799, %790
  %295 = phi i8 [ 1, %790 ], [ 1, %1350 ], [ %800, %799 ], [ 1, %817 ], [ 1, %813 ], [ 1, %.loopexit.split-lp ], [ 1, %.loopexit106 ]
  %296 = phi { ptr, i32 } [ %791, %790 ], [ %791, %1350 ], [ %801, %799 ], [ %814, %817 ], [ %814, %813 ], [ %lpad.loopexit.split-lp, %.loopexit.split-lp ], [ %lpad.loopexit, %.loopexit106 ]
  %297 = icmp eq i64 %243, 0
  br i1 %297, label %1445, label %298

298:                                              ; preds = %294
  %299 = shl nuw i64 %243, 2
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %242, i64 noundef %299, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !38953
  br label %1445

.loopexit106:                                     ; preds = %.loopexit104, %345, %340, %353, %360
  %lpad.loopexit = landingpad { ptr, i32 }
          cleanup
  br label %294

.loopexit.split-lp:                               ; preds = %356
  %lpad.loopexit.split-lp = landingpad { ptr, i32 }
          cleanup
  br label %294

300:                                              ; preds = %810, %289
  %301 = phi ptr [ %290, %289 ], [ %561, %810 ]
  %302 = phi i64 [ %291, %289 ], [ %562, %810 ]
  %303 = phi ptr [ %293, %289 ], [ %304, %810 ]
  %304 = getelementptr inbounds nuw i8, ptr %303, i64 4
  %305 = load i32, ptr %303, align 4, !range !3837, !noalias !38956, !noundef !1740
  br i1 %244, label %340, label %.loopexit104

.loopexit108:                                     ; preds = %.loopexit105, %810, %856, %247
  %306 = phi i64 [ %302, %856 ], [ %562, %810 ], [ 0, %247 ], [ %302, %.loopexit105 ]
  %307 = phi i1 [ %292, %856 ], [ %292, %810 ], [ false, %247 ], [ true, %.loopexit105 ]
  %308 = phi i1 [ false, %856 ], [ true, %810 ], [ true, %247 ], [ true, %.loopexit105 ]
  %309 = icmp eq i64 %243, 0
  br i1 %309, label %862, label %310

310:                                              ; preds = %.loopexit108
  %311 = shl nuw i64 %243, 2
  %312 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %313 = load i64, ptr %312, align 8, !noalias !38959, !noundef !1740
  %314 = call i64 @llvm.umin.i64(i64 %311, i64 9223372036854775807)
  %315 = call i64 @llvm.ssub.sat.i64(i64 %313, i64 %314)
  store i64 %315, ptr %312, align 8, !noalias !38959
  %316 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %317 = load i64, ptr %316, align 8, !noalias !38959, !noundef !1740
  %318 = icmp slt i64 %315, %317
  br i1 %318, label %319, label %.preheader1087

319:                                              ; preds = %310
  store i64 %315, ptr %316, align 8, !noalias !38959
  br label %.preheader1087

.preheader1087:                                   ; preds = %319, %310
  br label %320

320:                                              ; preds = %.preheader1087, %323
  %321 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !38959
  %322 = icmp slt i64 %321, 0
  br i1 %322, label %323, label %__rustc::__rust_dealloc (.exit)

323:                                              ; preds = %320
  %324 = add nsw i64 %321, 1
  %325 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %321, i64 %324 acq_rel acquire, align 8, !noalias !38959
  %326 = extractvalue { i64, i1 } %325, 1
  br i1 %326, label %327, label %320

327:                                              ; preds = %323
  %328 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %314 monotonic, align 8, !noalias !38959
  %329 = call i64 @llvm.ssub.sat.i64(i64 %328, i64 %314)
  %330 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !38959
  br label %331

331:                                              ; preds = %334, %327
  %332 = phi i64 [ %330, %327 ], [ %337, %334 ]
  %333 = icmp slt i64 %329, %332
  br i1 %333, label %334, label %338

334:                                              ; preds = %331
  %335 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %332, i64 %329 monotonic monotonic, align 8, !noalias !38959
  %336 = extractvalue { i64, i1 } %335, 1
  %337 = extractvalue { i64, i1 } %335, 0
  br i1 %336, label %338, label %331

338:                                              ; preds = %334, %331
  %339 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !38959
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %320, %338
  call void @free(ptr noundef nonnull %242) #93, !noalias !38959
  br label %862

.loopexit104:                                     ; preds = %378, %362, %347, %300
  store i32 2, ptr %248, align 16, !alias.scope !38898, !noalias !38901
  store i32 %305, ptr %250, align 4, !alias.scope !38898, !noalias !38901
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %46, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %3)
          to label %386 unwind label %.loopexit106, !inline_history !38962

340:                                              ; preds = %300
  %341 = load ptr, ptr %71, align 8, !alias.scope !38898, !noalias !38901, !nonnull !1740, !align !1836, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !38963
; call <purrdf_core::ir::dataset::RdfDataset>::probe_plan
  %342 = call { i64, i8 } @<purrdf_core::ir::dataset::RdfDataset>::probe_plan(i1 noundef zeroext false, i1 noundef zeroext false, i1 noundef zeroext false, i32 noundef 2, i32 poison)
  %343 = extractvalue { i64, i8 } %342, 0
  %344 = extractvalue { i64, i8 } %342, 1
  store i64 %343, ptr %8, align 8, !noalias !38963
  store i8 %344, ptr %283, align 8, !noalias !38963
; invoke <purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan
  invoke void @<purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan(ptr noalias nofree noundef nonnull sret([88 x i8]) align 8 captures(none) dereferenceable(88) %19, ptr noundef nonnull align 8 %341, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %8, i32 noundef 0, i32 noundef 0, i32 noundef 0, i32 noundef 2, i32 range(i32 1, 0) %305)
          to label %345 unwind label %.loopexit106

345:                                              ; preds = %340
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !38963
; invoke <purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next
  %346 = invoke fastcc noundef align 4 ptr @<purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next(ptr noalias nofree noundef nonnull align 8 dereferenceable(88) %19) #88
          to label %347 unwind label %.loopexit106

347:                                              ; preds = %345
  %348 = icmp eq ptr %346, null
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !38952
  br i1 %348, label %349, label %.loopexit104

349:                                              ; preds = %347
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !38952
  call void @llvm.experimental.noalias.scope.decl(metadata !38966)
  %350 = getelementptr inbounds nuw i8, ptr %341, i64 80
  %351 = load i64, ptr %350, align 8, !noalias !38966, !noundef !1740
  %352 = icmp eq i64 %351, 0
  br i1 %352, label %360, label %353

353:                                              ; preds = %349
; invoke <purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri
  %354 = invoke noundef i32 @<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri(ptr noundef nonnull readonly align 8 %341, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158, i64 noundef 50)
          to label %.noexc unwind label %.loopexit106

.noexc:                                           ; preds = %353
  %355 = icmp eq i32 %354, 0
  br i1 %355, label %356, label %360, !prof !1742

356:                                              ; preds = %.noexc
  %357 = getelementptr inbounds nuw i8, ptr %341, i64 80
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !38966
  %358 = load i64, ptr %357, align 8, !noalias !38966, !noundef !1740
  store i64 %358, ptr %7, align 8, !noalias !38966
  call void @llvm.lifetime.start.p0(ptr nonnull %6), !noalias !38966
  store ptr %7, ptr %6, align 8, !noalias !38966
  %359 = getelementptr inbounds nuw i8, ptr %6, i64 8
  store ptr @<usize as core::fmt::Display>::fmt, ptr %359, align 8, !noalias !38966
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.4c9120963acf3a9d820f038c36ebab01.4165.llvm.6298868053391388158, ptr noundef nonnull %6, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.4c9120963acf3a9d820f038c36ebab01.4166.llvm.6298868053391388158) #89
          to label %.noexc73 unwind label %.loopexit.split-lp

.noexc73:                                         ; preds = %356
  unreachable

360:                                              ; preds = %.noexc, %349
; invoke <purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri
  %361 = invoke noundef i32 @<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri(ptr noundef nonnull readonly align 8 %341, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158, i64 noundef 50)
          to label %362 unwind label %.loopexit106

362:                                              ; preds = %360
  store ptr %341, ptr %259, align 8, !alias.scope !38966
  store i32 %361, ptr %284, align 8, !alias.scope !38966
  store ptr null, ptr %285, align 8, !alias.scope !38966
  store ptr null, ptr %286, align 8, !alias.scope !38966
  store i32 2, ptr %17, align 8, !alias.scope !38969, !noalias !38974
  store i32 %305, ptr %260, align 4, !alias.scope !38969, !noalias !38974
; call <core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next
  call fastcc void @<core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next(ptr noalias nofree noundef align 4 captures(address) dereferenceable(16) %18, ptr noalias nofree noundef align 8 dereferenceable(72) %17) #88, !noalias !38924, !inline_history !38900
  %363 = load i32, ptr %18, align 4, !noalias !38952, !noundef !1740
  %364 = icmp eq i32 %363, 0
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !38952
  br i1 %364, label %365, label %.loopexit104

365:                                              ; preds = %362
  %366 = getelementptr i8, ptr %341, i64 88
  %367 = load ptr, ptr %366, align 8, !noalias !38924, !nonnull !1740, !noundef !1740
  %368 = getelementptr i8, ptr %341, i64 96
  %369 = load i64, ptr %368, align 8, !noalias !38924, !noundef !1740
  %370 = shl nuw nsw i64 %369, 4
  %371 = getelementptr inbounds nuw i8, ptr %367, i64 %370
  %372 = icmp eq i64 %369, 0
  br i1 %372, label %.loopexit105, label %.preheader103

.preheader103:                                    ; preds = %365, %383
  %373 = phi ptr [ %374, %383 ], [ %367, %365 ]
  %374 = getelementptr inbounds nuw i8, ptr %373, i64 16
  %375 = getelementptr inbounds nuw i8, ptr %373, i64 12
  %376 = load i32, ptr %375, align 4, !alias.scope !38976, !noalias !38979
  %377 = icmp eq i32 %376, 0
  br i1 %377, label %383, label %378

378:                                              ; preds = %.preheader103
  %379 = load i32, ptr %373, align 4, !alias.scope !38976, !noalias !38979
  %380 = icmp ne i32 %376, %305
  %381 = icmp eq i32 %379, 0
  %382 = select i1 %380, i1 true, i1 %381
  br i1 %382, label %383, label %.loopexit104

383:                                              ; preds = %378, %.preheader103
  %384 = icmp eq ptr %374, %371
  br i1 %384, label %.loopexit105, label %.preheader103

.loopexit105:                                     ; preds = %365, %383
  %385 = icmp eq ptr %304, %256
  br i1 %385, label %.loopexit108, label %289

386:                                              ; preds = %.loopexit104
  %387 = load i64, ptr %46, align 16, !range !1739, !noundef !1740
  %388 = trunc nuw i64 %387 to i1
  br i1 %388, label %389, label %391

389:                                              ; preds = %386
  %390 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %390, ptr noundef nonnull align 16 dereferenceable(96) %262, i64 96, i1 false)
  store i32 %249, ptr %248, align 16, !alias.scope !38898, !noalias !38901
  store i32 %251, ptr %250, align 4, !alias.scope !38898, !noalias !38901
  store i64 1, ptr %0, align 16, !alias.scope !38893, !noalias !38996
  br label %1319

391:                                              ; preds = %386
  %392 = load i64, ptr %261, align 8, !range !2059, !noundef !1740
  %393 = icmp eq i64 %392, -1
  br i1 %393, label %395, label %394

394:                                              ; preds = %391
  call void @llvm.lifetime.start.p0(ptr nonnull %45), !noalias !38952
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %45, ptr noalias nofree noundef align 8 dereferenceable(104) %38, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %261)
          to label %852 unwind label %799

395:                                              ; preds = %391
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %47, ptr noundef nonnull align 16 dereferenceable(32) %262, i64 32, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %43)
  call void @llvm.lifetime.start.p0(ptr nonnull %42)
  call void @llvm.experimental.noalias.scope.decl(metadata !38997)
  call void @llvm.lifetime.start.p0(ptr nonnull %26), !noalias !39000
  %396 = load ptr, ptr %263, align 8, !alias.scope !38997, !noalias !39004, !nonnull !1740, !noundef !1740
  %397 = getelementptr inbounds nuw i8, ptr %396, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %26, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %397)
          to label %407 unwind label %405, !noalias !39005, !inline_history !38900

398:                                              ; preds = %412, %405
  %399 = phi ptr [ %413, %412 ], [ %396, %405 ]
  %400 = phi i1 [ %410, %412 ], [ true, %405 ]
  %401 = phi { ptr, i32 } [ %411, %412 ], [ %406, %405 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !39006)
  call void @llvm.experimental.noalias.scope.decl(metadata !39009), !noalias !39012
  %402 = atomicrmw sub ptr %399, i64 1 release, align 8, !noalias !39013
  %403 = icmp eq i64 %402, 1
  br i1 %403, label %404, label %784

404:                                              ; preds = %398
  fence acquire, !noalias !39012
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %263) #92
          to label %784 unwind label %608

405:                                              ; preds = %395
  %406 = landingpad { ptr, i32 }
          cleanup
  br label %398

407:                                              ; preds = %395
; invoke <purrdf_sparql_eval::solution::VarSchema>::index_of
  %408 = invoke fastcc { i64, i64 } @<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.11631829254914579133)(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %26, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %63)
          to label %416 unwind label %414, !noalias !39014, !inline_history !38900

409:                                              ; preds = %616, %454, %414
  %410 = phi i1 [ false, %616 ], [ true, %414 ], [ false, %454 ]
  %411 = phi { ptr, i32 } [ %617, %616 ], [ %415, %414 ], [ %455, %454 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(56) %26) #90
          to label %412 unwind label %608, !noalias !39015, !inline_history !38900

412:                                              ; preds = %409
  %413 = load ptr, ptr %263, align 8, !alias.scope !39016, !noalias !39012
  br label %398

414:                                              ; preds = %432, %407
  %415 = landingpad { ptr, i32 }
          cleanup
  br label %409

416:                                              ; preds = %407
  %417 = extractvalue { i64, i64 } %408, 0
  %418 = extractvalue { i64, i64 } %408, 1
  %419 = trunc nuw i64 %417 to i1
  br i1 %419, label %420, label %429

420:                                              ; preds = %416
  %421 = load ptr, ptr %268, align 8, !alias.scope !38997, !noalias !39004, !nonnull !1740, !noundef !1740
  %422 = load i64, ptr %47, align 8, !range !1835, !alias.scope !38997, !noalias !39004, !noundef !1740
  %423 = load i64, ptr %269, align 8, !alias.scope !38997, !noalias !39004, !noundef !1740
  %424 = icmp ult i64 %423, 230584300921369396
  call void @llvm.assume(i1 %424)
  %425 = mul nuw nsw i64 %423, 40
  %426 = getelementptr inbounds nuw i8, ptr %421, i64 %425
  call void @llvm.lifetime.start.p0(ptr nonnull %25), !noalias !39000
  store ptr %421, ptr %25, align 8, !noalias !39000
  store i64 %422, ptr %277, align 8, !noalias !39000
  store ptr %426, ptr %278, align 8, !noalias !39000
  %427 = icmp eq i64 %423, 0
  br i1 %427, label %.loopexit98, label %.preheader97.preheader

.preheader97.preheader:                           ; preds = %420
  %428 = insertelement <2 x i32> <i32 0, i32 poison>, i32 %305, i64 1
  br label %.preheader97

429:                                              ; preds = %416
  %430 = atomicrmw add ptr %264, i64 1 monotonic, align 8, !noalias !39017
  %431 = icmp slt i64 %430, 0
  br i1 %431, label %434, label %432

432:                                              ; preds = %429
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %433 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %26, ptr noundef nonnull %264, i64 noundef %266)
          to label %435 unwind label %414, !noalias !39017, !inline_history !38900

434:                                              ; preds = %429
  call void @llvm.trap()
  unreachable

435:                                              ; preds = %432
  %436 = load i64, ptr %267, align 8, !noalias !39000, !noundef !1740
  %437 = icmp ult i64 %436, 576460752303423488
  call void @llvm.assume(i1 %437)
  %438 = load ptr, ptr %268, align 8, !alias.scope !38997, !noalias !39004, !nonnull !1740, !noundef !1740
  %439 = load i64, ptr %47, align 8, !range !1835, !alias.scope !38997, !noalias !39004, !noundef !1740
  %440 = load i64, ptr %269, align 8, !alias.scope !38997, !noalias !39004, !noundef !1740
  %441 = icmp ult i64 %440, 230584300921369396
  call void @llvm.assume(i1 %441)
  %442 = mul nuw nsw i64 %440, 40
  %443 = getelementptr inbounds nuw i8, ptr %438, i64 %442
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !39000
  store ptr %438, ptr %23, align 8, !noalias !39000
  store i64 %439, ptr %271, align 8, !noalias !39000
  store ptr %443, ptr %272, align 8, !noalias !39000
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  %444 = icmp eq i64 %440, 0
  br i1 %444, label %.loopexit102, label %445

445:                                              ; preds = %435
  %446 = add nuw nsw i64 %436, 1
  %447 = insertelement <2 x i32> <i32 0, i32 poison>, i32 %305, i64 1
  br label %456

448:                                              ; preds = %606, %587
  %449 = phi i64 [ %604, %606 ], [ %577, %587 ]
  %450 = phi ptr [ %607, %606 ], [ %578, %587 ]
  %451 = phi { ptr, i32 } [ %603, %606 ], [ %585, %587 ]
  %452 = shl i64 %449, 3
  %453 = add i64 %452, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %450, i64 noundef %453, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !38924
  br label %454

454:                                              ; preds = %602, %584, %448
  %455 = phi { ptr, i32 } [ %585, %584 ], [ %603, %602 ], [ %451, %448 ]
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %23) #90, !noalias !39015, !inline_history !38900
  br label %409

456:                                              ; preds = %590, %445
  %457 = phi ptr [ %301, %445 ], [ %591, %590 ]
  %458 = phi i64 [ %302, %445 ], [ %595, %590 ]
  %459 = phi ptr [ %438, %445 ], [ %460, %590 ]
  %460 = getelementptr inbounds nuw i8, ptr %459, i64 40
  %461 = load i64, ptr %459, align 8, !noalias !39018
  %462 = getelementptr inbounds nuw i8, ptr %459, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %22, ptr noundef nonnull align 8 dereferenceable(32) %462, i64 32, i1 false), !noalias !39018
  %463 = icmp eq i64 %461, 0
  br i1 %463, label %.loopexit102, label %464

464:                                              ; preds = %456
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !39000
  store i64 %461, ptr %21, align 8, !noalias !39000
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %273, ptr noundef nonnull align 8 dereferenceable(32) %22, i64 32, i1 false), !noalias !39000
  call void @llvm.experimental.noalias.scope.decl(metadata !39021)
  %465 = add i64 %461, -1
  %466 = icmp ugt i64 %465, 4
  %467 = load i64, ptr %274, align 8, !alias.scope !39021, !noalias !39015
  %468 = add i64 %467, -1
  %469 = select i1 %466, i64 %468, i64 %465
  %470 = icmp ugt i64 %436, %469
  br i1 %470, label %478, label %471

471:                                              ; preds = %464
  %472 = icmp ugt i64 %461, 5
  %473 = select i1 %472, i64 %467, i64 %461
  %474 = add i64 %473, -1
  %475 = icmp ult i64 %436, %474
  br i1 %475, label %476, label %566

476:                                              ; preds = %471
  %477 = select i1 %472, ptr %274, ptr %21
  store i64 %446, ptr %477, align 8, !alias.scope !39024, !noalias !39015
  br label %566

478:                                              ; preds = %464
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !39027
  %479 = sub nuw nsw i64 %436, %469
  store i32 2, ptr %10, align 8, !noalias !39027
  store i64 %479, ptr %275, align 8, !noalias !39027
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::Extend<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::extend::<core::iter::sources::repeat_n::RepeatN<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %21, ptr noalias nofree noundef align 8 captures(address) dereferenceable(16) %10)
          to label %480 unwind label %598

480:                                              ; preds = %478
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !39027
  br label %566

.loopexit102:                                     ; preds = %456, %435
  %481 = phi ptr [ %301, %435 ], [ %457, %456 ]
  %482 = phi i64 [ %302, %435 ], [ %458, %456 ]
  %483 = phi ptr [ %438, %435 ], [ %460, %456 ]
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  %484 = ptrtoint ptr %443 to i64
  %485 = ptrtoint ptr %483 to i64
  %486 = sub nuw i64 %484, %485
  %487 = udiv exact i64 %486, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !39028), !noalias !39015
  %488 = icmp eq ptr %443, %483
  br i1 %488, label %.loopexit101, label %.preheader100

.preheader100:                                    ; preds = %.loopexit102, %526
  %489 = phi i64 [ %491, %526 ], [ 0, %.loopexit102 ]
  %490 = getelementptr inbounds nuw [40 x i8], ptr %483, i64 %489
  %491 = add nuw nsw i64 %489, 1
  %492 = load i64, ptr %490, align 8, !range !1778, !alias.scope !39031, !noalias !39034, !noundef !1740
  %493 = icmp ugt i64 %492, 5
  br i1 %493, label %494, label %526

494:                                              ; preds = %.preheader100
  %495 = getelementptr i8, ptr %490, i64 8
  %496 = load ptr, ptr %495, align 8, !alias.scope !39028, !noalias !39034, !nonnull !1740, !noundef !1740
  %497 = shl i64 %492, 3
  %498 = add i64 %497, -8
  %499 = load i64, ptr %287, align 8, !noalias !39039, !noundef !1740
  %500 = call i64 @llvm.umin.i64(i64 %498, i64 9223372036854775807)
  %501 = call i64 @llvm.ssub.sat.i64(i64 %499, i64 %500)
  store i64 %501, ptr %287, align 8, !noalias !39039
  %502 = load i64, ptr %288, align 8, !noalias !39039, !noundef !1740
  %503 = icmp slt i64 %501, %502
  br i1 %503, label %504, label %.preheader1091

504:                                              ; preds = %494
  store i64 %501, ptr %288, align 8, !noalias !39039
  br label %.preheader1091

.preheader1091:                                   ; preds = %504, %494
  br label %505

505:                                              ; preds = %.preheader1091, %508
  %506 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39039
  %507 = icmp slt i64 %506, 0
  br i1 %507, label %508, label %__rustc::__rust_dealloc (.exit75)

508:                                              ; preds = %505
  %509 = add nsw i64 %506, 1
  %510 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %506, i64 %509 acq_rel acquire, align 8, !noalias !39039
  %511 = extractvalue { i64, i1 } %510, 1
  br i1 %511, label %512, label %505

512:                                              ; preds = %508
  %513 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %500 monotonic, align 8, !noalias !39039
  %514 = call i64 @llvm.ssub.sat.i64(i64 %513, i64 %500)
  %515 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39039
  br label %516

516:                                              ; preds = %519, %512
  %517 = phi i64 [ %515, %512 ], [ %522, %519 ]
  %518 = icmp slt i64 %514, %517
  br i1 %518, label %519, label %523

519:                                              ; preds = %516
  %520 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %517, i64 %514 monotonic monotonic, align 8, !noalias !39039
  %521 = extractvalue { i64, i1 } %520, 1
  %522 = extractvalue { i64, i1 } %520, 0
  br i1 %521, label %523, label %516

523:                                              ; preds = %519, %516
  %524 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39039
  br label %__rustc::__rust_dealloc (.exit75)

__rustc::__rust_dealloc (.exit75): ; preds = %505, %523
  %525 = icmp ne i64 %498, 0
  call void @llvm.assume(i1 %525), !noalias !39039
  call void @free(ptr noundef nonnull %496) #93, !noalias !39039
  br label %526

526:                                              ; preds = %__rustc::__rust_dealloc (.exit75), %.preheader100
  %527 = icmp eq i64 %491, %487
  br i1 %527, label %.loopexit101, label %.preheader100

.loopexit101:                                     ; preds = %526, %597, %.loopexit102
  %528 = phi i64 [ %595, %597 ], [ %482, %.loopexit102 ], [ %482, %526 ]
  %529 = phi ptr [ %591, %597 ], [ %481, %.loopexit102 ], [ %481, %526 ]
  %530 = icmp eq i64 %439, 0
  br i1 %530, label %559, label %531

531:                                              ; preds = %.loopexit101
  %532 = mul nuw i64 %439, 40
  %533 = load i64, ptr %287, align 8, !noalias !39034, !noundef !1740
  %534 = call i64 @llvm.umin.i64(i64 %532, i64 9223372036854775807)
  %535 = call i64 @llvm.ssub.sat.i64(i64 %533, i64 %534)
  store i64 %535, ptr %287, align 8, !noalias !39034
  %536 = load i64, ptr %288, align 8, !noalias !39034, !noundef !1740
  %537 = icmp slt i64 %535, %536
  br i1 %537, label %538, label %.preheader1095

538:                                              ; preds = %531
  store i64 %535, ptr %288, align 8, !noalias !39034
  br label %.preheader1095

.preheader1095:                                   ; preds = %538, %531
  br label %539

539:                                              ; preds = %.preheader1095, %542
  %540 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39034
  %541 = icmp slt i64 %540, 0
  br i1 %541, label %542, label %__rustc::__rust_dealloc (.exit76)

542:                                              ; preds = %539
  %543 = add nsw i64 %540, 1
  %544 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %540, i64 %543 acq_rel acquire, align 8, !noalias !39034
  %545 = extractvalue { i64, i1 } %544, 1
  br i1 %545, label %546, label %539

546:                                              ; preds = %542
  %547 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %534 monotonic, align 8, !noalias !39034
  %548 = call i64 @llvm.ssub.sat.i64(i64 %547, i64 %534)
  %549 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39034
  br label %550

550:                                              ; preds = %553, %546
  %551 = phi i64 [ %549, %546 ], [ %556, %553 ]
  %552 = icmp slt i64 %548, %551
  br i1 %552, label %553, label %557

553:                                              ; preds = %550
  %554 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %551, i64 %548 monotonic monotonic, align 8, !noalias !39034
  %555 = extractvalue { i64, i1 } %554, 1
  %556 = extractvalue { i64, i1 } %554, 0
  br i1 %555, label %557, label %550

557:                                              ; preds = %553, %550
  %558 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39034
  br label %__rustc::__rust_dealloc (.exit76)

__rustc::__rust_dealloc (.exit76): ; preds = %539, %557
  call void @free(ptr noundef nonnull %438) #93, !noalias !39034
  br label %559

559:                                              ; preds = %__rustc::__rust_dealloc (.exit76), %.loopexit101
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !39000
  br label %560

560:                                              ; preds = %711, %559
  %561 = phi ptr [ %681, %711 ], [ %529, %559 ]
  %562 = phi i64 [ %680, %711 ], [ %528, %559 ]
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %42, ptr noundef nonnull align 8 dereferenceable(56) %26, i64 56, i1 false), !noalias !39042
  call void @llvm.lifetime.end.p0(ptr nonnull %26), !noalias !39000
  %563 = atomicrmw sub ptr %396, i64 1 release, align 8, !noalias !39043
  %564 = icmp eq i64 %563, 1
  br i1 %564, label %565, label %786

565:                                              ; preds = %560
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %263) #92
          to label %786 unwind label %797

566:                                              ; preds = %480, %476, %471
  %567 = load i64, ptr %21, align 8, !range !1778, !noalias !39000, !noundef !1740
  %568 = icmp ugt i64 %567, 5
  %569 = load i64, ptr %274, align 8
  %570 = select i1 %568, i64 %569, i64 %567
  %571 = add i64 %570, -1
  %572 = icmp ult i64 %433, %571
  br i1 %572, label %573, label %588

573:                                              ; preds = %566
  %574 = load ptr, ptr %273, align 8, !noalias !39000, !nonnull !1740
  %575 = select i1 %568, ptr %574, ptr %273
  %576 = getelementptr inbounds nuw [8 x i8], ptr %575, i64 %433
  store <2 x i32> %447, ptr %576, align 4, !noalias !39015
  call void @llvm.lifetime.start.p0(ptr nonnull %20)
  %577 = load i64, ptr %21, align 8, !noalias !39000
  %578 = load ptr, ptr %273, align 8, !noalias !39000
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %20, ptr noundef nonnull align 8 dereferenceable(24) %274, i64 24, i1 false), !noalias !39000
  call void @llvm.experimental.noalias.scope.decl(metadata !39048)
  %579 = load i64, ptr %48, align 8, !range !1835, !alias.scope !39048, !noalias !39051, !noundef !1740
  %580 = icmp eq i64 %458, %579
  br i1 %580, label %581, label %590

581:                                              ; preds = %573
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %48)
          to label %582 unwind label %584, !noalias !39051

582:                                              ; preds = %581
  %583 = load ptr, ptr %252, align 8, !alias.scope !39048, !noalias !39051
  br label %590

584:                                              ; preds = %581
  %585 = landingpad { ptr, i32 }
          cleanup
  store ptr %460, ptr %270, align 8
  %586 = icmp ugt i64 %577, 5
  br i1 %586, label %587, label %454

587:                                              ; preds = %584
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %578) ]
  br label %448

588:                                              ; preds = %566
  store ptr %460, ptr %270, align 8
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %433, i64 noundef %571, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.537) #94
          to label %589 unwind label %600, !noalias !39015, !inline_history !38900

589:                                              ; preds = %588
  unreachable

590:                                              ; preds = %582, %573
  %591 = phi ptr [ %583, %582 ], [ %457, %573 ]
  %592 = getelementptr inbounds nuw [40 x i8], ptr %591, i64 %458
  store i64 %577, ptr %592, align 8, !noalias !39053
  %593 = getelementptr inbounds nuw i8, ptr %592, i64 8
  store ptr %578, ptr %593, align 8, !noalias !39053
  %594 = getelementptr inbounds nuw i8, ptr %592, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %594, ptr noundef nonnull align 8 dereferenceable(24) %20, i64 24, i1 false), !noalias !39053
  %595 = add i64 %458, 1
  store i64 %595, ptr %253, align 8, !alias.scope !39048, !noalias !39051
  call void @llvm.lifetime.end.p0(ptr nonnull %20)
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !39000
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  %596 = icmp eq ptr %460, %443
  br i1 %596, label %597, label %456

597:                                              ; preds = %590
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  br label %.loopexit101

598:                                              ; preds = %478
  %599 = landingpad { ptr, i32 }
          cleanup
  store ptr %460, ptr %270, align 8
  br label %602

600:                                              ; preds = %588
  %601 = landingpad { ptr, i32 }
          cleanup
  br label %602

602:                                              ; preds = %600, %598
  %603 = phi { ptr, i32 } [ %599, %598 ], [ %601, %600 ]
  %604 = load i64, ptr %21, align 8, !range !1778, !alias.scope !39054, !noalias !38952, !noundef !1740
  %605 = icmp ugt i64 %604, 5
  br i1 %605, label %606, label %454

606:                                              ; preds = %602
  %607 = load ptr, ptr %273, align 8, !noalias !38952, !nonnull !1740, !noundef !1740
  br label %448

608:                                              ; preds = %409, %404
  %609 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !39012, !inline_history !38900
  unreachable

610:                                              ; preds = %781, %772
  %611 = phi i64 [ %764, %772 ], [ %622, %781 ]
  %612 = phi ptr [ %765, %772 ], [ %628, %781 ]
  %613 = phi { ptr, i32 } [ %770, %772 ], [ %782, %781 ]
  %614 = shl i64 %611, 3
  %615 = add i64 %614, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %612, i64 noundef %615, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !38924
  br label %616

616:                                              ; preds = %781, %769, %610
  %617 = phi { ptr, i32 } [ %770, %769 ], [ %782, %781 ], [ %613, %610 ]
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %25) #90, !noalias !39015, !inline_history !38900
  br label %409

.preheader97:                                     ; preds = %.preheader97.preheader, %757
  %618 = phi ptr [ %758, %757 ], [ %301, %.preheader97.preheader ]
  %619 = phi i64 [ %759, %757 ], [ %302, %.preheader97.preheader ]
  %620 = phi ptr [ %621, %757 ], [ %421, %.preheader97.preheader ]
  %621 = getelementptr inbounds nuw i8, ptr %620, i64 40
  %622 = load i64, ptr %620, align 8, !noalias !39057
  %623 = icmp eq i64 %622, 0
  br i1 %623, label %.loopexit98, label %624

624:                                              ; preds = %.preheader97
  %625 = getelementptr inbounds nuw i8, ptr %620, i64 8
  store i64 %622, ptr %24, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %279, ptr noundef nonnull align 8 dereferenceable(32) %625, i64 32, i1 false)
  %626 = add i64 %622, -1
  %627 = icmp ugt i64 %626, 4
  %628 = load ptr, ptr %279, align 8
  %629 = load i64, ptr %280, align 8
  %630 = add i64 %629, -1
  %631 = select i1 %627, i64 %630, i64 %626
  %632 = icmp ult i64 %418, %631
  br i1 %632, label %712, label %.invoke

.loopexit98:                                      ; preds = %.preheader97, %420
  %633 = phi ptr [ %301, %420 ], [ %618, %.preheader97 ]
  %634 = phi i64 [ %302, %420 ], [ %619, %.preheader97 ]
  %635 = phi ptr [ %421, %420 ], [ %621, %.preheader97 ]
  %636 = ptrtoint ptr %426 to i64
  %637 = ptrtoint ptr %635 to i64
  %638 = sub nuw i64 %636, %637
  %639 = udiv exact i64 %638, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !39060), !noalias !39015
  %640 = icmp eq ptr %426, %635
  br i1 %640, label %.loopexit96, label %.preheader95

.preheader95:                                     ; preds = %.loopexit98, %678
  %641 = phi i64 [ %643, %678 ], [ 0, %.loopexit98 ]
  %642 = getelementptr inbounds nuw [40 x i8], ptr %635, i64 %641
  %643 = add nuw nsw i64 %641, 1
  %644 = load i64, ptr %642, align 8, !range !1778, !alias.scope !39063, !noalias !39066, !noundef !1740
  %645 = icmp ugt i64 %644, 5
  br i1 %645, label %646, label %678

646:                                              ; preds = %.preheader95
  %647 = getelementptr i8, ptr %642, i64 8
  %648 = load ptr, ptr %647, align 8, !alias.scope !39060, !noalias !39066, !nonnull !1740, !noundef !1740
  %649 = shl i64 %644, 3
  %650 = add i64 %649, -8
  %651 = load i64, ptr %287, align 8, !noalias !39071, !noundef !1740
  %652 = call i64 @llvm.umin.i64(i64 %650, i64 9223372036854775807)
  %653 = call i64 @llvm.ssub.sat.i64(i64 %651, i64 %652)
  store i64 %653, ptr %287, align 8, !noalias !39071
  %654 = load i64, ptr %288, align 8, !noalias !39071, !noundef !1740
  %655 = icmp slt i64 %653, %654
  br i1 %655, label %656, label %.preheader1089

656:                                              ; preds = %646
  store i64 %653, ptr %288, align 8, !noalias !39071
  br label %.preheader1089

.preheader1089:                                   ; preds = %656, %646
  br label %657

657:                                              ; preds = %.preheader1089, %660
  %658 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39071
  %659 = icmp slt i64 %658, 0
  br i1 %659, label %660, label %__rustc::__rust_dealloc (.exit77)

660:                                              ; preds = %657
  %661 = add nsw i64 %658, 1
  %662 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %658, i64 %661 acq_rel acquire, align 8, !noalias !39071
  %663 = extractvalue { i64, i1 } %662, 1
  br i1 %663, label %664, label %657

664:                                              ; preds = %660
  %665 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %652 monotonic, align 8, !noalias !39071
  %666 = call i64 @llvm.ssub.sat.i64(i64 %665, i64 %652)
  %667 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39071
  br label %668

668:                                              ; preds = %671, %664
  %669 = phi i64 [ %667, %664 ], [ %674, %671 ]
  %670 = icmp slt i64 %666, %669
  br i1 %670, label %671, label %675

671:                                              ; preds = %668
  %672 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %669, i64 %666 monotonic monotonic, align 8, !noalias !39071
  %673 = extractvalue { i64, i1 } %672, 1
  %674 = extractvalue { i64, i1 } %672, 0
  br i1 %673, label %675, label %668

675:                                              ; preds = %671, %668
  %676 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39071
  br label %__rustc::__rust_dealloc (.exit77)

__rustc::__rust_dealloc (.exit77): ; preds = %657, %675
  %677 = icmp ne i64 %650, 0
  call void @llvm.assume(i1 %677), !noalias !39071
  call void @free(ptr noundef nonnull %648) #93, !noalias !39071
  br label %678

678:                                              ; preds = %__rustc::__rust_dealloc (.exit77), %.preheader95
  %679 = icmp eq i64 %643, %639
  br i1 %679, label %.loopexit96, label %.preheader95

.loopexit96:                                      ; preds = %757, %678, %.loopexit98
  %680 = phi i64 [ %634, %678 ], [ %634, %.loopexit98 ], [ %759, %757 ]
  %681 = phi ptr [ %633, %678 ], [ %633, %.loopexit98 ], [ %758, %757 ]
  %682 = icmp eq i64 %422, 0
  br i1 %682, label %711, label %683

683:                                              ; preds = %.loopexit96
  %684 = mul nuw i64 %422, 40
  %685 = load i64, ptr %287, align 8, !noalias !39066, !noundef !1740
  %686 = call i64 @llvm.umin.i64(i64 %684, i64 9223372036854775807)
  %687 = call i64 @llvm.ssub.sat.i64(i64 %685, i64 %686)
  store i64 %687, ptr %287, align 8, !noalias !39066
  %688 = load i64, ptr %288, align 8, !noalias !39066, !noundef !1740
  %689 = icmp slt i64 %687, %688
  br i1 %689, label %690, label %.preheader1093

690:                                              ; preds = %683
  store i64 %687, ptr %288, align 8, !noalias !39066
  br label %.preheader1093

.preheader1093:                                   ; preds = %690, %683
  br label %691

691:                                              ; preds = %.preheader1093, %694
  %692 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39066
  %693 = icmp slt i64 %692, 0
  br i1 %693, label %694, label %__rustc::__rust_dealloc (.exit78)

694:                                              ; preds = %691
  %695 = add nsw i64 %692, 1
  %696 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %692, i64 %695 acq_rel acquire, align 8, !noalias !39066
  %697 = extractvalue { i64, i1 } %696, 1
  br i1 %697, label %698, label %691

698:                                              ; preds = %694
  %699 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %686 monotonic, align 8, !noalias !39066
  %700 = call i64 @llvm.ssub.sat.i64(i64 %699, i64 %686)
  %701 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39066
  br label %702

702:                                              ; preds = %705, %698
  %703 = phi i64 [ %701, %698 ], [ %708, %705 ]
  %704 = icmp slt i64 %700, %703
  br i1 %704, label %705, label %709

705:                                              ; preds = %702
  %706 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %703, i64 %700 monotonic monotonic, align 8, !noalias !39066
  %707 = extractvalue { i64, i1 } %706, 1
  %708 = extractvalue { i64, i1 } %706, 0
  br i1 %707, label %709, label %702

709:                                              ; preds = %705, %702
  %710 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39066
  br label %__rustc::__rust_dealloc (.exit78)

__rustc::__rust_dealloc (.exit78): ; preds = %691, %709
  call void @free(ptr noundef nonnull %421) #93, !noalias !39066
  br label %711

711:                                              ; preds = %__rustc::__rust_dealloc (.exit78), %.loopexit96
  call void @llvm.lifetime.end.p0(ptr nonnull %25), !noalias !39000
  br label %560

712:                                              ; preds = %624
  %713 = select i1 %627, ptr %628, ptr %279
  %714 = getelementptr inbounds nuw [8 x i8], ptr %713, i64 %418
  %715 = load i32, ptr %714, align 4, !range !1785, !noundef !1740
  switch i32 %715, label %721 [
    i32 2, label %716
    i32 0, label %753
  ]

716:                                              ; preds = %753, %712
  %717 = icmp ugt i64 %622, 5
  %718 = select i1 %717, i64 %629, i64 %622
  %719 = add i64 %718, -1
  %720 = icmp ult i64 %418, %719
  br i1 %720, label %761, label %.invoke

721:                                              ; preds = %753, %712
  %722 = icmp ugt i64 %622, 5
  br i1 %722, label %723, label %757

723:                                              ; preds = %721
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %628) ], !noalias !38924
  %724 = shl i64 %622, 3
  %725 = add i64 %724, -8
  %726 = load i64, ptr %287, align 8, !noalias !39074, !noundef !1740
  %727 = call i64 @llvm.umin.i64(i64 %725, i64 9223372036854775807)
  %728 = call i64 @llvm.ssub.sat.i64(i64 %726, i64 %727)
  store i64 %728, ptr %287, align 8, !noalias !39074
  %729 = load i64, ptr %288, align 8, !noalias !39074, !noundef !1740
  %730 = icmp slt i64 %728, %729
  br i1 %730, label %731, label %.preheader1090

731:                                              ; preds = %723
  store i64 %728, ptr %288, align 8, !noalias !39074
  br label %.preheader1090

.preheader1090:                                   ; preds = %731, %723
  br label %732

732:                                              ; preds = %.preheader1090, %735
  %733 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39074
  %734 = icmp slt i64 %733, 0
  br i1 %734, label %735, label %__rustc::__rust_dealloc (.exit79)

735:                                              ; preds = %732
  %736 = add nsw i64 %733, 1
  %737 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %733, i64 %736 acq_rel acquire, align 8, !noalias !39074
  %738 = extractvalue { i64, i1 } %737, 1
  br i1 %738, label %739, label %732

739:                                              ; preds = %735
  %740 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %727 monotonic, align 8, !noalias !39074
  %741 = call i64 @llvm.ssub.sat.i64(i64 %740, i64 %727)
  %742 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39074
  br label %743

743:                                              ; preds = %746, %739
  %744 = phi i64 [ %742, %739 ], [ %749, %746 ]
  %745 = icmp slt i64 %741, %744
  br i1 %745, label %746, label %750

746:                                              ; preds = %743
  %747 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %744, i64 %741 monotonic monotonic, align 8, !noalias !39074
  %748 = extractvalue { i64, i1 } %747, 1
  %749 = extractvalue { i64, i1 } %747, 0
  br i1 %748, label %750, label %743

750:                                              ; preds = %746, %743
  %751 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39074
  br label %__rustc::__rust_dealloc (.exit79)

__rustc::__rust_dealloc (.exit79): ; preds = %732, %750
  %752 = icmp ne i64 %725, 0
  call void @llvm.assume(i1 %752), !noalias !39074
  call void @free(ptr noundef nonnull %628) #93, !noalias !39074
  br label %757

753:                                              ; preds = %712
  %754 = getelementptr inbounds nuw i8, ptr %714, i64 4
  %755 = load i32, ptr %754, align 4, !range !3837, !noundef !1740
  %756 = icmp eq i32 %755, %305
  br i1 %756, label %716, label %721

757:                                              ; preds = %775, %__rustc::__rust_dealloc (.exit79), %721
  %758 = phi ptr [ %618, %__rustc::__rust_dealloc (.exit79) ], [ %618, %721 ], [ %776, %775 ]
  %759 = phi i64 [ %619, %__rustc::__rust_dealloc (.exit79) ], [ %619, %721 ], [ %780, %775 ]
  %760 = icmp eq ptr %621, %426
  br i1 %760, label %.loopexit96, label %.preheader97

761:                                              ; preds = %716
  %762 = select i1 %717, ptr %628, ptr %279
  %763 = getelementptr inbounds nuw [8 x i8], ptr %762, i64 %418
  store <2 x i32> %428, ptr %763, align 4
  %764 = load i64, ptr %24, align 8
  %765 = load ptr, ptr %279, align 8
  call void @llvm.experimental.noalias.scope.decl(metadata !39077)
  %766 = load i64, ptr %48, align 8, !range !1835, !alias.scope !39077, !noalias !39080, !noundef !1740
  %767 = icmp eq i64 %619, %766
  br i1 %767, label %768, label %775

768:                                              ; preds = %761
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %48)
          to label %775 unwind label %769, !noalias !39080

769:                                              ; preds = %768
  %770 = landingpad { ptr, i32 }
          cleanup
  store ptr %621, ptr %276, align 8
  %771 = icmp ugt i64 %764, 5
  br i1 %771, label %772, label %616

772:                                              ; preds = %769
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %765) ]
  br label %610

.invoke:                                          ; preds = %716, %624
  %773 = phi i64 [ %631, %624 ], [ %719, %716 ]
  %774 = phi ptr [ @anon.e5162873a9a3251d11c4df37a70e4654.538, %624 ], [ @anon.e5162873a9a3251d11c4df37a70e4654.539, %716 ]
  store ptr %621, ptr %276, align 8
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %418, i64 noundef %773, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) %774) #94
          to label %.cont unwind label %781, !noalias !39015, !inline_history !38900

.cont:                                            ; preds = %.invoke
  unreachable

775:                                              ; preds = %768, %761
  %776 = load ptr, ptr %252, align 8, !alias.scope !39077, !noalias !39080, !nonnull !1740, !noundef !1740
  %777 = getelementptr inbounds nuw [40 x i8], ptr %776, i64 %619
  store i64 %764, ptr %777, align 8, !noalias !39082
  %778 = getelementptr inbounds nuw i8, ptr %777, i64 8
  store ptr %765, ptr %778, align 8, !noalias !39082
  %779 = getelementptr inbounds nuw i8, ptr %777, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %779, ptr noundef nonnull align 8 dereferenceable(24) %280, i64 24, i1 false)
  %780 = add i64 %619, 1
  store i64 %780, ptr %253, align 8, !alias.scope !39077, !noalias !39080
  br label %757

781:                                              ; preds = %.invoke
  %782 = landingpad { ptr, i32 }
          cleanup
  %783 = icmp ugt i64 %622, 5
  br i1 %783, label %610, label %616

784:                                              ; preds = %404, %398
  br i1 %400, label %785, label %790

785:                                              ; preds = %784
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %47) #90, !noalias !39012, !inline_history !38900
  br label %790

786:                                              ; preds = %565, %560
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %43, ptr noundef nonnull align 8 dereferenceable(56) %42, i64 56, i1 false), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %42)
  %787 = load i64, ptr %49, align 8, !range !2059, !alias.scope !39083, !noalias !38924, !noundef !1740
  %788 = icmp eq i64 %787, -1
  br i1 %788, label %804, label %789

789:                                              ; preds = %786
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(56) %49)
          to label %804 unwind label %802

790:                                              ; preds = %802, %797, %785, %784
  %791 = phi { ptr, i32 } [ %803, %802 ], [ %401, %784 ], [ %401, %785 ], [ %798, %797 ]
  %792 = load i64, ptr %46, align 16, !range !1739, !noundef !1740
  %793 = icmp eq i64 %792, 0
  %794 = load i64, ptr %261, align 8, !range !2059
  %795 = icmp ne i64 %794, -1
  %796 = select i1 %793, i1 %795, i1 false
  br i1 %796, label %1350, label %294

797:                                              ; preds = %565
  %798 = landingpad { ptr, i32 }
          cleanup
  br label %790

799:                                              ; preds = %1316, %855, %394
  %800 = phi i8 [ 1, %855 ], [ 1, %394 ], [ 0, %1316 ]
  %801 = landingpad { ptr, i32 }
          cleanup
  br label %294

802:                                              ; preds = %789
  %803 = landingpad { ptr, i32 }
          cleanup
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %49, ptr noundef nonnull align 8 dereferenceable(56) %43, i64 56, i1 false), !noalias !38952
  br label %790

804:                                              ; preds = %789, %786
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %49, ptr noundef nonnull align 8 dereferenceable(56) %43, i64 56, i1 false), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %43)
  %805 = load i64, ptr %46, align 16, !range !1739, !noundef !1740
  %806 = trunc nuw i64 %805 to i1
  %807 = load i64, ptr %261, align 8, !range !2059
  %808 = icmp eq i64 %807, -1
  %809 = select i1 %806, i1 true, i1 %808
  br i1 %809, label %810, label %812

810:                                              ; preds = %__rustc::__rust_dealloc (.exit80), %820, %804
  %811 = icmp eq ptr %304, %256
  br i1 %811, label %.loopexit108, label %300

812:                                              ; preds = %804
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(96) %261)
          to label %820 unwind label %813

813:                                              ; preds = %812
  %814 = landingpad { ptr, i32 }
          cleanup
  %815 = load i64, ptr %281, align 8
  %816 = icmp eq i64 %815, 0
  br i1 %816, label %294, label %817

817:                                              ; preds = %813
  %818 = load ptr, ptr %282, align 16, !nonnull !1740, !noundef !1740
  %819 = mul nuw i64 %815, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %818, i64 noundef %819, i64 noundef range(i64 1, -9223372036854775807) 1) #93, !noalias !39086
  br label %294

820:                                              ; preds = %812
  %821 = load i64, ptr %281, align 8
  %822 = icmp eq i64 %821, 0
  br i1 %822, label %810, label %823

823:                                              ; preds = %820
  %824 = load ptr, ptr %282, align 16, !nonnull !1740, !noundef !1740
  %825 = mul nuw i64 %821, 3
  %826 = load i64, ptr %287, align 8, !noalias !39086, !noundef !1740
  %827 = call i64 @llvm.umin.i64(i64 %825, i64 9223372036854775807)
  %828 = call i64 @llvm.ssub.sat.i64(i64 %826, i64 %827)
  store i64 %828, ptr %287, align 8, !noalias !39086
  %829 = load i64, ptr %288, align 8, !noalias !39086, !noundef !1740
  %830 = icmp slt i64 %828, %829
  br i1 %830, label %831, label %.preheader1092

831:                                              ; preds = %823
  store i64 %828, ptr %288, align 8, !noalias !39086
  br label %.preheader1092

.preheader1092:                                   ; preds = %831, %823
  br label %832

832:                                              ; preds = %.preheader1092, %835
  %833 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39086
  %834 = icmp slt i64 %833, 0
  br i1 %834, label %835, label %__rustc::__rust_dealloc (.exit80)

835:                                              ; preds = %832
  %836 = add nsw i64 %833, 1
  %837 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %833, i64 %836 acq_rel acquire, align 8, !noalias !39086
  %838 = extractvalue { i64, i1 } %837, 1
  br i1 %838, label %839, label %832

839:                                              ; preds = %835
  %840 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %827 monotonic, align 8, !noalias !39086
  %841 = call i64 @llvm.ssub.sat.i64(i64 %840, i64 %827)
  %842 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39086
  br label %843

843:                                              ; preds = %846, %839
  %844 = phi i64 [ %842, %839 ], [ %849, %846 ]
  %845 = icmp slt i64 %841, %844
  br i1 %845, label %846, label %850

846:                                              ; preds = %843
  %847 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %844, i64 %841 monotonic monotonic, align 8, !noalias !39086
  %848 = extractvalue { i64, i1 } %847, 1
  %849 = extractvalue { i64, i1 } %847, 0
  br i1 %848, label %850, label %843

850:                                              ; preds = %846, %843
  %851 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39086
  br label %__rustc::__rust_dealloc (.exit80)

__rustc::__rust_dealloc (.exit80): ; preds = %832, %850
  call void @free(ptr noundef nonnull %824) #93, !noalias !39086
  br label %810

852:                                              ; preds = %394
  %853 = load i64, ptr %45, align 8, !range !2059, !noalias !38952, !noundef !1740
  %854 = icmp eq i64 %853, -1
  br i1 %854, label %1316, label %855

855:                                              ; preds = %852
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %45)
          to label %856 unwind label %799

856:                                              ; preds = %855
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !38952
  br label %.loopexit108

857:                                              ; preds = %1242, %1236, %961, %939, %930, %923, %921
  %858 = phi i8 [ 0, %939 ], [ 1, %961 ], [ 1, %921 ], [ 1, %1236 ], [ 1, %1242 ], [ 1, %930 ], [ 1, %923 ]
  %859 = landingpad { ptr, i32 }
          cleanup
  br label %1445

860:                                              ; preds = %1016
  %861 = landingpad { ptr, i32 }
          cleanup
  br label %1351

862:                                              ; preds = %__rustc::__rust_dealloc (.exit), %.loopexit108
  store i32 %249, ptr %248, align 16, !alias.scope !38898, !noalias !38901
  store i32 %251, ptr %250, align 4, !alias.scope !38898, !noalias !38901
  %863 = load i64, ptr %49, align 8, !range !2059, !noalias !38952, !noundef !1740
  %864 = icmp eq i64 %863, -1
  br i1 %864, label %911, label %865

865:                                              ; preds = %862
  %866 = getelementptr inbounds nuw i8, ptr %30, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %30), !noalias !38952
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %866, ptr noundef nonnull align 8 dereferenceable(56) %49, i64 56, i1 false), !noalias !38952
  store i64 1, ptr %30, align 8, !noalias !38952
  %867 = getelementptr inbounds nuw i8, ptr %30, i64 8
  store i64 1, ptr %867, align 8, !noalias !38952
  %868 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #93, !noalias !39091
  %869 = icmp eq ptr %868, null
  br i1 %869, label %__rustc::__rust_alloc (.exit81.thread), label %870

870:                                              ; preds = %865
  %871 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %872 = load i64, ptr %871, align 8, !noalias !39091, !noundef !1740
  %873 = call i64 @llvm.uadd.sat.i64(i64 %872, i64 1)
  store i64 %873, ptr %871, align 8, !noalias !39091
  %874 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %875 = load i64, ptr %874, align 8, !noalias !39091, !noundef !1740
  %876 = call i64 @llvm.uadd.sat.i64(i64 %875, i64 72)
  store i64 %876, ptr %874, align 8, !noalias !39091
  %877 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %878 = load i64, ptr %877, align 8, !noalias !39091, !noundef !1740
  %879 = call i64 @llvm.sadd.sat.i64(i64 %878, i64 72)
  store i64 %879, ptr %877, align 8, !noalias !39091
  %880 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %881 = load i64, ptr %880, align 8, !noalias !39091, !noundef !1740
  %882 = icmp sgt i64 %879, %881
  br i1 %882, label %883, label %.preheader1086

883:                                              ; preds = %870
  store i64 %879, ptr %880, align 8, !noalias !39091
  br label %.preheader1086

.preheader1086:                                   ; preds = %883, %870
  br label %884

884:                                              ; preds = %.preheader1086, %887
  %885 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39091
  %886 = icmp slt i64 %885, 0
  br i1 %886, label %887, label %__rustc::__rust_alloc (.exit81)

887:                                              ; preds = %884
  %888 = add nsw i64 %885, 1
  %889 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %885, i64 %888 acq_rel acquire, align 8, !noalias !39091
  %890 = extractvalue { i64, i1 } %889, 1
  br i1 %890, label %891, label %884

891:                                              ; preds = %887
  %892 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !39091
  %893 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !39091
  %894 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !39091
  %895 = call i64 @llvm.sadd.sat.i64(i64 %894, i64 72)
  %896 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !39091
  br label %897

897:                                              ; preds = %900, %891
  %898 = phi i64 [ %896, %891 ], [ %903, %900 ]
  %899 = icmp sgt i64 %895, %898
  br i1 %899, label %900, label %904

900:                                              ; preds = %897
  %901 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %898, i64 %895 monotonic monotonic, align 8, !noalias !39091
  %902 = extractvalue { i64, i1 } %901, 1
  %903 = extractvalue { i64, i1 } %901, 0
  br i1 %902, label %904, label %897

904:                                              ; preds = %900, %897
  %905 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39091
  br label %__rustc::__rust_alloc (.exit81)

__rustc::__rust_alloc (.exit81.thread): ; preds = %865
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #94
          to label %906 unwind label %907

906:                                              ; preds = %__rustc::__rust_alloc (.exit81.thread)
  unreachable

907:                                              ; preds = %__rustc::__rust_alloc (.exit81.thread)
  %908 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %866)
          to label %1445 unwind label %909

909:                                              ; preds = %907
  %910 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924, !inline_history !38900
  unreachable

__rustc::__rust_alloc (.exit81): ; preds = %884, %904
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %868, ptr noundef nonnull align 8 dereferenceable(72) %30, i64 72, i1 false), !noalias !38924
  call void @llvm.lifetime.end.p0(ptr nonnull %30), !noalias !38952
  br label %963

911:                                              ; preds = %862
  br i1 %308, label %912, label %913

912:                                              ; preds = %911
  br i1 %307, label %921, label %923

913:                                              ; preds = %911
  %914 = getelementptr inbounds nuw i8, ptr %38, i64 96
  %915 = load ptr, ptr %914, align 8, !noundef !1740
  %916 = icmp eq ptr %915, null
  br i1 %916, label %1242, label %917

917:                                              ; preds = %913
  %918 = atomicrmw add ptr %915, i64 1 monotonic, align 8, !noalias !38924
  %919 = icmp slt i64 %918, 0
  br i1 %919, label %920, label %1170

920:                                              ; preds = %917
  call void @llvm.trap(), !noalias !38924
  unreachable

921:                                              ; preds = %912
  call void @llvm.lifetime.start.p0(ptr nonnull %41), !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %40), !noalias !38952
; invoke purrdf_sparql_eval::eval::syntactic_schema
  %922 = invoke noundef nonnull ptr @purrdf_sparql_eval::eval::syntactic_schema(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3)
          to label %1102 unwind label %857, !noalias !38924, !inline_history !38900

923:                                              ; preds = %912
  call void @llvm.lifetime.start.p0(ptr nonnull %36), !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %35)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %28, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %3)
          to label %924 unwind label %857, !inline_history !38962

924:                                              ; preds = %923
  %925 = load i64, ptr %28, align 16, !range !1739, !noundef !1740
  %926 = trunc nuw i64 %925 to i1
  br i1 %926, label %927, label %930

927:                                              ; preds = %924
  %928 = getelementptr inbounds nuw i8, ptr %28, i64 16
  %929 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %929, ptr noundef nonnull align 16 dereferenceable(96) %928, i64 96, i1 false)
  store i64 1, ptr %0, align 16, !alias.scope !38893, !noalias !38996
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !38952
  br label %1023

930:                                              ; preds = %924
  %931 = getelementptr inbounds nuw i8, ptr %28, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %35, ptr noundef nonnull align 8 dereferenceable(96) %931, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %36, ptr noalias nofree noundef align 8 dereferenceable(104) %38, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %35)
          to label %932 unwind label %857

932:                                              ; preds = %930
  %933 = load i64, ptr %36, align 8, !range !2059, !noalias !38952, !noundef !1740
  %934 = icmp eq i64 %933, -1
  br i1 %934, label %939, label %935

935:                                              ; preds = %932
  call void @llvm.lifetime.start.p0(ptr nonnull %37), !noalias !38952
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %37, ptr noundef nonnull align 8 dereferenceable(32) %36, i64 32, i1 false), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %34), !noalias !38952
  %936 = getelementptr inbounds nuw i8, ptr %37, i64 24
  %937 = load ptr, ptr %936, align 8, !noalias !38952, !nonnull !1740, !noundef !1740
  %938 = getelementptr inbounds nuw i8, ptr %937, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %34, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %938)
          to label %944 unwind label %942, !noalias !38924, !inline_history !38900

939:                                              ; preds = %932
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  call void @llvm.lifetime.end.p0(ptr nonnull %36), !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %39), !noalias !38952
; invoke <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %39, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %38)
          to label %1021 unwind label %857

940:                                              ; preds = %1017, %957, %942
  %941 = phi { ptr, i32 } [ %1018, %1017 ], [ %943, %942 ], [ %958, %957 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %37) #90
          to label %1445 unwind label %1019, !noalias !38924, !inline_history !38900

942:                                              ; preds = %935
  %943 = landingpad { ptr, i32 }
          cleanup
  br label %940

944:                                              ; preds = %935
  %945 = load ptr, ptr %63, align 8, !alias.scope !38896, !noalias !39094, !nonnull !1740, !noundef !1740
  %946 = atomicrmw add ptr %945, i64 1 monotonic, align 8, !noalias !38924
  %947 = icmp slt i64 %946, 0
  br i1 %947, label %952, label %948

948:                                              ; preds = %944
  %949 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %950 = load i64, ptr %949, align 8, !alias.scope !38896, !noalias !39094, !noundef !1740
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %951 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %34, ptr noundef nonnull %945, i64 noundef %950)
          to label %953 unwind label %1017, !noalias !38924, !inline_history !38900

952:                                              ; preds = %944
  call void @llvm.trap()
  unreachable

953:                                              ; preds = %948
  %954 = getelementptr inbounds nuw i8, ptr %27, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %27), !noalias !38952
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %954, ptr noundef nonnull align 8 dereferenceable(56) %34, i64 56, i1 false), !noalias !38952
  store i64 1, ptr %27, align 8, !noalias !38952
  %955 = getelementptr inbounds nuw i8, ptr %27, i64 8
  store i64 1, ptr %955, align 8, !noalias !38952
; invoke alloc::boxed::box_new_uninit
  %956 = invoke fastcc noundef ptr @alloc::boxed::box_new_uninit(i64 noundef 8, i64 noundef 72)
          to label %961 unwind label %957, !noalias !39095, !inline_history !38900

957:                                              ; preds = %953
  %958 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %954)
          to label %940 unwind label %959

959:                                              ; preds = %957
  %960 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924, !inline_history !38900
  unreachable

961:                                              ; preds = %953
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %956, ptr noundef nonnull align 8 dereferenceable(72) %27, i64 72, i1 false), !noalias !38924
  call void @llvm.lifetime.end.p0(ptr nonnull %27), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %34), !noalias !38952
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %37)
          to label %962 unwind label %857, !noalias !38924, !inline_history !38900

962:                                              ; preds = %961
  call void @llvm.lifetime.end.p0(ptr nonnull %37), !noalias !38952
  br label %963

963:                                              ; preds = %__rustc::__rust_alloc (.exit87), %1241, %__rustc::__rust_alloc (.exit85), %962, %__rustc::__rust_alloc (.exit81)
  %964 = phi ptr [ %868, %__rustc::__rust_alloc (.exit81) ], [ %956, %962 ], [ %1125, %__rustc::__rust_alloc (.exit85) ], [ %1191, %1241 ], [ %1267, %__rustc::__rust_alloc (.exit87) ]
  call void @llvm.lifetime.start.p0(ptr nonnull %33), !noalias !38952
  call void @llvm.lifetime.start.p0(ptr nonnull %32), !noalias !38952
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %32, ptr noundef nonnull align 8 dereferenceable(104) %38, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %31), !noalias !38952
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %31, ptr noundef nonnull align 8 dereferenceable(24) %48, i64 24, i1 false), !noalias !38952
  %965 = getelementptr inbounds nuw i8, ptr %31, i64 24
  store ptr %964, ptr %965, align 8, !noalias !38952
  call void @llvm.experimental.noalias.scope.decl(metadata !39098)
  call void @llvm.experimental.noalias.scope.decl(metadata !39101)
  call void @llvm.experimental.noalias.scope.decl(metadata !39103)
  %966 = load i64, ptr %32, align 8, !range !2059, !alias.scope !39101, !noalias !39105, !noundef !1740
  %967 = icmp eq i64 %966, -1
  br i1 %967, label %969, label %968

968:                                              ; preds = %963
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %33, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %31, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %38)
  br label %971

969:                                              ; preds = %963
  %970 = getelementptr inbounds nuw i8, ptr %33, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %970, ptr noundef nonnull readonly align 8 dereferenceable(32) %31, i64 32, i1 false), !alias.scope !39106, !noalias !39107
  store i64 -1, ptr %33, align 8, !alias.scope !39098, !noalias !39108
  br label %971

971:                                              ; preds = %969, %968
  %972 = getelementptr inbounds nuw i8, ptr %32, i64 72
  %973 = load i64, ptr %972, align 8, !range !1778, !alias.scope !39109, !noalias !39105, !noundef !1740
  %974 = icmp ugt i64 %973, 5
  br i1 %974, label %975, label %1009

975:                                              ; preds = %971
  %976 = getelementptr inbounds nuw i8, ptr %32, i64 80
  %977 = load ptr, ptr %976, align 8, !alias.scope !39101, !noalias !39105, !nonnull !1740, !noundef !1740
  %978 = mul i64 %973, 3
  %979 = add i64 %978, -3
  %980 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %981 = load i64, ptr %980, align 8, !noalias !39112, !noundef !1740
  %982 = call i64 @llvm.umin.i64(i64 %979, i64 9223372036854775807)
  %983 = call i64 @llvm.ssub.sat.i64(i64 %981, i64 %982)
  store i64 %983, ptr %980, align 8, !noalias !39112
  %984 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %985 = load i64, ptr %984, align 8, !noalias !39112, !noundef !1740
  %986 = icmp slt i64 %983, %985
  br i1 %986, label %987, label %.preheader1078

987:                                              ; preds = %975
  store i64 %983, ptr %984, align 8, !noalias !39112
  br label %.preheader1078

.preheader1078:                                   ; preds = %987, %975
  br label %988

988:                                              ; preds = %.preheader1078, %991
  %989 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39112
  %990 = icmp slt i64 %989, 0
  br i1 %990, label %991, label %__rustc::__rust_dealloc (.exit82)

991:                                              ; preds = %988
  %992 = add nsw i64 %989, 1
  %993 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %989, i64 %992 acq_rel acquire, align 8, !noalias !39112
  %994 = extractvalue { i64, i1 } %993, 1
  br i1 %994, label %995, label %988

995:                                              ; preds = %991
  %996 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %982 monotonic, align 8, !noalias !39112
  %997 = call i64 @llvm.ssub.sat.i64(i64 %996, i64 %982)
  %998 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39112
  br label %999

999:                                              ; preds = %1002, %995
  %1000 = phi i64 [ %998, %995 ], [ %1005, %1002 ]
  %1001 = icmp slt i64 %997, %1000
  br i1 %1001, label %1002, label %1006

1002:                                             ; preds = %999
  %1003 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1000, i64 %997 monotonic monotonic, align 8, !noalias !39112
  %1004 = extractvalue { i64, i1 } %1003, 1
  %1005 = extractvalue { i64, i1 } %1003, 0
  br i1 %1004, label %1006, label %999

1006:                                             ; preds = %1002, %999
  %1007 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39112
  br label %__rustc::__rust_dealloc (.exit82)

__rustc::__rust_dealloc (.exit82): ; preds = %988, %1006
  %1008 = icmp ne i64 %979, 0
  call void @llvm.assume(i1 %1008), !noalias !39112
  call void @free(ptr noundef nonnull %977) #93, !noalias !39112
  br label %1009

1009:                                             ; preds = %__rustc::__rust_dealloc (.exit82), %971
  %1010 = getelementptr inbounds nuw i8, ptr %32, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !39115), !noalias !38924
  %1011 = load ptr, ptr %1010, align 8, !alias.scope !39118, !noalias !39105, !noundef !1740
  %1012 = icmp eq ptr %1011, null
  br i1 %1012, label %1314, label %1013

1013:                                             ; preds = %1009
  %1014 = atomicrmw sub ptr %1011, i64 1 release, align 8, !noalias !39119
  %1015 = icmp eq i64 %1014, 1
  br i1 %1015, label %1016, label %1314

1016:                                             ; preds = %1013
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1010) #92
          to label %1314 unwind label %860

1017:                                             ; preds = %948
  %1018 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(56) %34) #90
          to label %940 unwind label %1019, !noalias !38924, !inline_history !38900

1019:                                             ; preds = %1458, %1450, %1350, %1168, %1108, %1017, %940
  %1020 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924, !inline_history !38900
  unreachable

1021:                                             ; preds = %939
  %1022 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1022, ptr noundef nonnull align 8 dereferenceable(96) %39, i64 96, i1 false), !noalias !38996
  store i64 0, ptr %0, align 16, !alias.scope !38893, !noalias !38996
  call void @llvm.lifetime.end.p0(ptr nonnull %39), !noalias !38952
  br label %1023

1023:                                             ; preds = %__rustc::__rust_dealloc (.exit88), %1319, %1021, %927
  %1024 = phi i64 [ %306, %1021 ], [ %306, %927 ], [ %302, %__rustc::__rust_dealloc (.exit88) ], [ %302, %1319 ]
  %1025 = phi i8 [ 0, %1021 ], [ 1, %927 ], [ %1320, %__rustc::__rust_dealloc (.exit88) ], [ %1320, %1319 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !39124)
  %1026 = load ptr, ptr %252, align 8, !alias.scope !39124, !noalias !38924, !nonnull !1740, !noundef !1740
  call void @llvm.experimental.noalias.scope.decl(metadata !39127), !noalias !38924
  %1027 = icmp eq i64 %1024, 0
  br i1 %1027, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %1023
  %1028 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1029 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %1030

1030:                                             ; preds = %.preheader, %1068
  %1031 = phi i64 [ %1033, %1068 ], [ 0, %.preheader ]
  %1032 = getelementptr inbounds nuw [40 x i8], ptr %1026, i64 %1031
  %1033 = add nuw nsw i64 %1031, 1
  %1034 = load i64, ptr %1032, align 8, !range !1778, !alias.scope !39130, !noalias !39133, !noundef !1740
  %1035 = icmp ugt i64 %1034, 5
  br i1 %1035, label %1036, label %1068

1036:                                             ; preds = %1030
  %1037 = getelementptr i8, ptr %1032, i64 8
  %1038 = load ptr, ptr %1037, align 8, !alias.scope !39127, !noalias !39133, !nonnull !1740, !noundef !1740
  %1039 = shl i64 %1034, 3
  %1040 = add i64 %1039, -8
  %1041 = load i64, ptr %1028, align 8, !noalias !39134, !noundef !1740
  %1042 = call i64 @llvm.umin.i64(i64 %1040, i64 9223372036854775807)
  %1043 = call i64 @llvm.ssub.sat.i64(i64 %1041, i64 %1042)
  store i64 %1043, ptr %1028, align 8, !noalias !39134
  %1044 = load i64, ptr %1029, align 8, !noalias !39134, !noundef !1740
  %1045 = icmp slt i64 %1043, %1044
  br i1 %1045, label %1046, label %.preheader1083

1046:                                             ; preds = %1036
  store i64 %1043, ptr %1029, align 8, !noalias !39134
  br label %.preheader1083

.preheader1083:                                   ; preds = %1046, %1036
  br label %1047

1047:                                             ; preds = %.preheader1083, %1050
  %1048 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39134
  %1049 = icmp slt i64 %1048, 0
  br i1 %1049, label %1050, label %__rustc::__rust_dealloc (.exit83)

1050:                                             ; preds = %1047
  %1051 = add nsw i64 %1048, 1
  %1052 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1048, i64 %1051 acq_rel acquire, align 8, !noalias !39134
  %1053 = extractvalue { i64, i1 } %1052, 1
  br i1 %1053, label %1054, label %1047

1054:                                             ; preds = %1050
  %1055 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1042 monotonic, align 8, !noalias !39134
  %1056 = call i64 @llvm.ssub.sat.i64(i64 %1055, i64 %1042)
  %1057 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39134
  br label %1058

1058:                                             ; preds = %1061, %1054
  %1059 = phi i64 [ %1057, %1054 ], [ %1064, %1061 ]
  %1060 = icmp slt i64 %1056, %1059
  br i1 %1060, label %1061, label %1065

1061:                                             ; preds = %1058
  %1062 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1059, i64 %1056 monotonic monotonic, align 8, !noalias !39134
  %1063 = extractvalue { i64, i1 } %1062, 1
  %1064 = extractvalue { i64, i1 } %1062, 0
  br i1 %1063, label %1065, label %1058

1065:                                             ; preds = %1061, %1058
  %1066 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39134
  br label %__rustc::__rust_dealloc (.exit83)

__rustc::__rust_dealloc (.exit83): ; preds = %1047, %1065
  %1067 = icmp ne i64 %1040, 0
  call void @llvm.assume(i1 %1067), !noalias !39134
  call void @free(ptr noundef nonnull %1038) #93, !noalias !39134
  br label %1068

1068:                                             ; preds = %__rustc::__rust_dealloc (.exit83), %1030
  %1069 = icmp eq i64 %1033, %1024
  br i1 %1069, label %.loopexit, label %1030

.loopexit:                                        ; preds = %1068, %1023
  %1070 = load i64, ptr %48, align 8, !alias.scope !39124, !noalias !38924
  %1071 = icmp eq i64 %1070, 0
  br i1 %1071, label %1358, label %1072

1072:                                             ; preds = %.loopexit
  %1073 = mul nuw i64 %1070, 40
  %1074 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1075 = load i64, ptr %1074, align 8, !noalias !39133, !noundef !1740
  %1076 = call i64 @llvm.umin.i64(i64 %1073, i64 9223372036854775807)
  %1077 = call i64 @llvm.ssub.sat.i64(i64 %1075, i64 %1076)
  store i64 %1077, ptr %1074, align 8, !noalias !39133
  %1078 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1079 = load i64, ptr %1078, align 8, !noalias !39133, !noundef !1740
  %1080 = icmp slt i64 %1077, %1079
  br i1 %1080, label %1081, label %.preheader1082

1081:                                             ; preds = %1072
  store i64 %1077, ptr %1078, align 8, !noalias !39133
  br label %.preheader1082

.preheader1082:                                   ; preds = %1081, %1072
  br label %1082

1082:                                             ; preds = %.preheader1082, %1085
  %1083 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39133
  %1084 = icmp slt i64 %1083, 0
  br i1 %1084, label %1085, label %__rustc::__rust_dealloc (.exit84)

1085:                                             ; preds = %1082
  %1086 = add nsw i64 %1083, 1
  %1087 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1083, i64 %1086 acq_rel acquire, align 8, !noalias !39133
  %1088 = extractvalue { i64, i1 } %1087, 1
  br i1 %1088, label %1089, label %1082

1089:                                             ; preds = %1085
  %1090 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1076 monotonic, align 8, !noalias !39133
  %1091 = call i64 @llvm.ssub.sat.i64(i64 %1090, i64 %1076)
  %1092 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39133
  br label %1093

1093:                                             ; preds = %1096, %1089
  %1094 = phi i64 [ %1092, %1089 ], [ %1099, %1096 ]
  %1095 = icmp slt i64 %1091, %1094
  br i1 %1095, label %1096, label %1100

1096:                                             ; preds = %1093
  %1097 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1094, i64 %1091 monotonic monotonic, align 8, !noalias !39133
  %1098 = extractvalue { i64, i1 } %1097, 1
  %1099 = extractvalue { i64, i1 } %1097, 0
  br i1 %1098, label %1100, label %1093

1100:                                             ; preds = %1096, %1093
  %1101 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39133
  br label %__rustc::__rust_dealloc (.exit84)

__rustc::__rust_dealloc (.exit84): ; preds = %1082, %1100
  call void @free(ptr noundef nonnull %1026) #93, !noalias !39133
  br label %1358

1102:                                             ; preds = %921
  store ptr %922, ptr %40, align 8, !noalias !38952
  %1103 = getelementptr inbounds nuw i8, ptr %922, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %41, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1103)
          to label %1109 unwind label %1104, !noalias !38924, !inline_history !38900

1104:                                             ; preds = %1102
  %1105 = landingpad { ptr, i32 }
          cleanup
  %1106 = atomicrmw sub ptr %922, i64 1 release, align 8, !noalias !39137
  %1107 = icmp eq i64 %1106, 1
  br i1 %1107, label %1108, label %1445

1108:                                             ; preds = %1104
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #92
          to label %1445 unwind label %1019

1109:                                             ; preds = %1102
  %1110 = atomicrmw sub ptr %922, i64 1 release, align 8, !noalias !39142
  %1111 = icmp eq i64 %1110, 1
  br i1 %1111, label %1112, label %1113

1112:                                             ; preds = %1109
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %40) #92
          to label %1113 unwind label %1168

1113:                                             ; preds = %1112, %1109
  call void @llvm.lifetime.end.p0(ptr nonnull %40), !noalias !38952
  %1114 = load ptr, ptr %63, align 8, !alias.scope !38896, !noalias !39094, !nonnull !1740, !noundef !1740
  %1115 = atomicrmw add ptr %1114, i64 1 monotonic, align 8, !noalias !38924
  %1116 = icmp slt i64 %1115, 0
  br i1 %1116, label %1121, label %1117

1117:                                             ; preds = %1113
  %1118 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %1119 = load i64, ptr %1118, align 8, !alias.scope !38896, !noalias !39094, !noundef !1740
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %1120 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %41, ptr noundef nonnull %1114, i64 noundef %1119)
          to label %1122 unwind label %1168, !noalias !38924, !inline_history !38900

1121:                                             ; preds = %1113
  call void @llvm.trap()
  unreachable

1122:                                             ; preds = %1117
  %1123 = getelementptr inbounds nuw i8, ptr %29, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %29), !noalias !38952
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %1123, ptr noundef nonnull align 8 dereferenceable(56) %41, i64 56, i1 false), !noalias !38952
  store i64 1, ptr %29, align 8, !noalias !38952
  %1124 = getelementptr inbounds nuw i8, ptr %29, i64 8
  store i64 1, ptr %1124, align 8, !noalias !38952
  %1125 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #93, !noalias !39147
  %1126 = icmp eq ptr %1125, null
  br i1 %1126, label %__rustc::__rust_alloc (.exit85.thread), label %1127

1127:                                             ; preds = %1122
  %1128 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1129 = load i64, ptr %1128, align 8, !noalias !39147, !noundef !1740
  %1130 = call i64 @llvm.uadd.sat.i64(i64 %1129, i64 1)
  store i64 %1130, ptr %1128, align 8, !noalias !39147
  %1131 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1132 = load i64, ptr %1131, align 8, !noalias !39147, !noundef !1740
  %1133 = call i64 @llvm.uadd.sat.i64(i64 %1132, i64 72)
  store i64 %1133, ptr %1131, align 8, !noalias !39147
  %1134 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1135 = load i64, ptr %1134, align 8, !noalias !39147, !noundef !1740
  %1136 = call i64 @llvm.sadd.sat.i64(i64 %1135, i64 72)
  store i64 %1136, ptr %1134, align 8, !noalias !39147
  %1137 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1138 = load i64, ptr %1137, align 8, !noalias !39147, !noundef !1740
  %1139 = icmp sgt i64 %1136, %1138
  br i1 %1139, label %1140, label %.preheader1079

1140:                                             ; preds = %1127
  store i64 %1136, ptr %1137, align 8, !noalias !39147
  br label %.preheader1079

.preheader1079:                                   ; preds = %1140, %1127
  br label %1141

1141:                                             ; preds = %.preheader1079, %1144
  %1142 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39147
  %1143 = icmp slt i64 %1142, 0
  br i1 %1143, label %1144, label %__rustc::__rust_alloc (.exit85)

1144:                                             ; preds = %1141
  %1145 = add nsw i64 %1142, 1
  %1146 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1142, i64 %1145 acq_rel acquire, align 8, !noalias !39147
  %1147 = extractvalue { i64, i1 } %1146, 1
  br i1 %1147, label %1148, label %1141

1148:                                             ; preds = %1144
  %1149 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !39147
  %1150 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !39147
  %1151 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !39147
  %1152 = call i64 @llvm.sadd.sat.i64(i64 %1151, i64 72)
  %1153 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !39147
  br label %1154

1154:                                             ; preds = %1157, %1148
  %1155 = phi i64 [ %1153, %1148 ], [ %1160, %1157 ]
  %1156 = icmp sgt i64 %1152, %1155
  br i1 %1156, label %1157, label %1161

1157:                                             ; preds = %1154
  %1158 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %1155, i64 %1152 monotonic monotonic, align 8, !noalias !39147
  %1159 = extractvalue { i64, i1 } %1158, 1
  %1160 = extractvalue { i64, i1 } %1158, 0
  br i1 %1159, label %1161, label %1154

1161:                                             ; preds = %1157, %1154
  %1162 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39147
  br label %__rustc::__rust_alloc (.exit85)

__rustc::__rust_alloc (.exit85.thread): ; preds = %1122
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #94
          to label %1163 unwind label %1164

1163:                                             ; preds = %__rustc::__rust_alloc (.exit85.thread)
  unreachable

1164:                                             ; preds = %__rustc::__rust_alloc (.exit85.thread)
  %1165 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %1123)
          to label %1445 unwind label %1166

1166:                                             ; preds = %1164
  %1167 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924, !inline_history !38900
  unreachable

__rustc::__rust_alloc (.exit85): ; preds = %1141, %1161
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1125, ptr noundef nonnull align 8 dereferenceable(72) %29, i64 72, i1 false), !noalias !38924
  call void @llvm.lifetime.end.p0(ptr nonnull %29), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %41), !noalias !38952
  br label %963

1168:                                             ; preds = %1117, %1112
  %1169 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(56) %41) #90
          to label %1445 unwind label %1019, !noalias !38924, !inline_history !38900

1170:                                             ; preds = %917
  %1171 = load ptr, ptr %63, align 8, !alias.scope !38896, !noalias !39094
  %1172 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %1173 = load i64, ptr %1172, align 8, !alias.scope !38896, !noalias !39094
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !38924
  store ptr %915, ptr %16, align 8, !noalias !38924
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !38924
  %1174 = getelementptr inbounds nuw i8, ptr %915, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %15, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1174)
          to label %1182 unwind label %1180, !noalias !38924

1175:                                             ; preds = %1237, %1230, %1180
  %1176 = phi { ptr, i32 } [ %1238, %1237 ], [ %1181, %1180 ], [ %1231, %1230 ]
  %1177 = atomicrmw sub ptr %915, i64 1 release, align 8, !noalias !39150
  %1178 = icmp eq i64 %1177, 1
  br i1 %1178, label %1179, label %1445

1179:                                             ; preds = %1175
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %16) #92
          to label %1445 unwind label %1239, !noalias !38924

1180:                                             ; preds = %1170
  %1181 = landingpad { ptr, i32 }
          cleanup
  br label %1175

1182:                                             ; preds = %1170
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1171) ], !noalias !38924
  %1183 = atomicrmw add ptr %1171, i64 1 monotonic, align 8, !noalias !38924
  %1184 = icmp slt i64 %1183, 0
  br i1 %1184, label %1187, label %1185

1185:                                             ; preds = %1182
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %1186 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %15, ptr noundef nonnull %1171, i64 noundef %1173)
          to label %1188 unwind label %1237, !noalias !38924

1187:                                             ; preds = %1182
  call void @llvm.trap(), !noalias !38924
  unreachable

1188:                                             ; preds = %1185
  %1189 = getelementptr inbounds nuw i8, ptr %14, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !38924
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %1189, ptr noundef nonnull align 8 dereferenceable(56) %15, i64 56, i1 false), !noalias !38924
  store i64 1, ptr %14, align 8, !noalias !38924
  %1190 = getelementptr inbounds nuw i8, ptr %14, i64 8
  store i64 1, ptr %1190, align 8, !noalias !38924
  %1191 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #93, !noalias !39155
  %1192 = icmp eq ptr %1191, null
  br i1 %1192, label %__rustc::__rust_alloc (.exit86.thread), label %1193

1193:                                             ; preds = %1188
  %1194 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1195 = load i64, ptr %1194, align 8, !noalias !39155, !noundef !1740
  %1196 = call i64 @llvm.uadd.sat.i64(i64 %1195, i64 1)
  store i64 %1196, ptr %1194, align 8, !noalias !39155
  %1197 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1198 = load i64, ptr %1197, align 8, !noalias !39155, !noundef !1740
  %1199 = call i64 @llvm.uadd.sat.i64(i64 %1198, i64 72)
  store i64 %1199, ptr %1197, align 8, !noalias !39155
  %1200 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1201 = load i64, ptr %1200, align 8, !noalias !39155, !noundef !1740
  %1202 = call i64 @llvm.sadd.sat.i64(i64 %1201, i64 72)
  store i64 %1202, ptr %1200, align 8, !noalias !39155
  %1203 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1204 = load i64, ptr %1203, align 8, !noalias !39155, !noundef !1740
  %1205 = icmp sgt i64 %1202, %1204
  br i1 %1205, label %1206, label %.preheader1085

1206:                                             ; preds = %1193
  store i64 %1202, ptr %1203, align 8, !noalias !39155
  br label %.preheader1085

.preheader1085:                                   ; preds = %1206, %1193
  br label %1207

1207:                                             ; preds = %.preheader1085, %1210
  %1208 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39155
  %1209 = icmp slt i64 %1208, 0
  br i1 %1209, label %1210, label %__rustc::__rust_alloc (.exit86)

1210:                                             ; preds = %1207
  %1211 = add nsw i64 %1208, 1
  %1212 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1208, i64 %1211 acq_rel acquire, align 8, !noalias !39155
  %1213 = extractvalue { i64, i1 } %1212, 1
  br i1 %1213, label %1214, label %1207

1214:                                             ; preds = %1210
  %1215 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !39155
  %1216 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !39155
  %1217 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !39155
  %1218 = call i64 @llvm.sadd.sat.i64(i64 %1217, i64 72)
  %1219 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !39155
  br label %1220

1220:                                             ; preds = %1223, %1214
  %1221 = phi i64 [ %1219, %1214 ], [ %1226, %1223 ]
  %1222 = icmp sgt i64 %1218, %1221
  br i1 %1222, label %1223, label %1227

1223:                                             ; preds = %1220
  %1224 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %1221, i64 %1218 monotonic monotonic, align 8, !noalias !39155
  %1225 = extractvalue { i64, i1 } %1224, 1
  %1226 = extractvalue { i64, i1 } %1224, 0
  br i1 %1225, label %1227, label %1220

1227:                                             ; preds = %1223, %1220
  %1228 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39155
  br label %__rustc::__rust_alloc (.exit86)

__rustc::__rust_alloc (.exit86.thread): ; preds = %1188
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #94
          to label %1229 unwind label %1230, !noalias !38924

1229:                                             ; preds = %__rustc::__rust_alloc (.exit86.thread)
  unreachable

1230:                                             ; preds = %__rustc::__rust_alloc (.exit86.thread)
  %1231 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %1189)
          to label %1175 unwind label %1232, !noalias !38924

1232:                                             ; preds = %1230
  %1233 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924
  unreachable

__rustc::__rust_alloc (.exit86): ; preds = %1207, %1227
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1191, ptr noundef nonnull align 8 dereferenceable(72) %14, i64 72, i1 false), !noalias !38924
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !38924
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !38924
  %1234 = atomicrmw sub ptr %915, i64 1 release, align 8, !noalias !39158
  %1235 = icmp eq i64 %1234, 1
  br i1 %1235, label %1236, label %1241

1236:                                             ; preds = %__rustc::__rust_alloc (.exit86)
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %16) #92
          to label %1241 unwind label %857

1237:                                             ; preds = %1185
  %1238 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(56) %15) #90
          to label %1175 unwind label %1239, !noalias !38924

1239:                                             ; preds = %1237, %1179
  %1240 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924
  unreachable

1241:                                             ; preds = %1236, %__rustc::__rust_alloc (.exit86)
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !38924
  br label %963

1242:                                             ; preds = %913
  %1243 = load ptr, ptr %63, align 8, !alias.scope !38896, !noalias !39094
  %1244 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %1245 = load i64, ptr %1244, align 8, !alias.scope !38896, !noalias !39094
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !38924
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !38924
; invoke purrdf_sparql_eval::eval::syntactic_schema
  %1246 = invoke noundef nonnull ptr @purrdf_sparql_eval::eval::syntactic_schema(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3)
          to label %1247 unwind label %857

1247:                                             ; preds = %1242
  store ptr %1246, ptr %12, align 8, !noalias !38924
  %1248 = getelementptr inbounds nuw i8, ptr %1246, i64 16
; invoke <purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone
  invoke fastcc void @<purrdf_sparql_eval::solution::VarSchema as core::clone::Clone>::clone (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(56) %13, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %1248)
          to label %1254 unwind label %1249, !noalias !38924

1249:                                             ; preds = %1247
  %1250 = landingpad { ptr, i32 }
          cleanup
  %1251 = atomicrmw sub ptr %1246, i64 1 release, align 8, !noalias !39163
  %1252 = icmp eq i64 %1251, 1
  br i1 %1252, label %1253, label %1445

1253:                                             ; preds = %1249
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %12) #92
          to label %1445 unwind label %1312, !noalias !38924

1254:                                             ; preds = %1247
  %1255 = atomicrmw sub ptr %1246, i64 1 release, align 8, !noalias !39168
  %1256 = icmp eq i64 %1255, 1
  br i1 %1256, label %1257, label %1258

1257:                                             ; preds = %1254
  fence acquire, !noalias !38924
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %12) #92
          to label %1258 unwind label %1310, !noalias !38924

1258:                                             ; preds = %1257, %1254
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !38924
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1243) ], !noalias !38924
  %1259 = atomicrmw add ptr %1243, i64 1 monotonic, align 8, !noalias !38924
  %1260 = icmp slt i64 %1259, 0
  br i1 %1260, label %1263, label %1261

1261:                                             ; preds = %1258
; invoke <purrdf_sparql_eval::solution::VarSchema>::push
  %1262 = invoke noundef i64 @<purrdf_sparql_eval::solution::VarSchema>::push(ptr noalias nofree noundef nonnull align 8 dereferenceable(56) %13, ptr noundef nonnull %1243, i64 noundef %1245)
          to label %1264 unwind label %1310, !noalias !38924

1263:                                             ; preds = %1258
  call void @llvm.trap(), !noalias !38924
  unreachable

1264:                                             ; preds = %1261
  %1265 = getelementptr inbounds nuw i8, ptr %11, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !38924
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(56) %1265, ptr noundef nonnull align 8 dereferenceable(56) %13, i64 56, i1 false), !noalias !38924
  store i64 1, ptr %11, align 8, !noalias !38924
  %1266 = getelementptr inbounds nuw i8, ptr %11, i64 8
  store i64 1, ptr %1266, align 8, !noalias !38924
  %1267 = call noundef dereferenceable_or_null(72) ptr @malloc(i64 noundef range(i64 1, 0) 72) #93, !noalias !39173
  %1268 = icmp eq ptr %1267, null
  br i1 %1268, label %__rustc::__rust_alloc (.exit87.thread), label %1269

1269:                                             ; preds = %1264
  %1270 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1271 = load i64, ptr %1270, align 8, !noalias !39173, !noundef !1740
  %1272 = call i64 @llvm.uadd.sat.i64(i64 %1271, i64 1)
  store i64 %1272, ptr %1270, align 8, !noalias !39173
  %1273 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1274 = load i64, ptr %1273, align 8, !noalias !39173, !noundef !1740
  %1275 = call i64 @llvm.uadd.sat.i64(i64 %1274, i64 72)
  store i64 %1275, ptr %1273, align 8, !noalias !39173
  %1276 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1277 = load i64, ptr %1276, align 8, !noalias !39173, !noundef !1740
  %1278 = call i64 @llvm.sadd.sat.i64(i64 %1277, i64 72)
  store i64 %1278, ptr %1276, align 8, !noalias !39173
  %1279 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1280 = load i64, ptr %1279, align 8, !noalias !39173, !noundef !1740
  %1281 = icmp sgt i64 %1278, %1280
  br i1 %1281, label %1282, label %.preheader1084

1282:                                             ; preds = %1269
  store i64 %1278, ptr %1279, align 8, !noalias !39173
  br label %.preheader1084

.preheader1084:                                   ; preds = %1282, %1269
  br label %1283

1283:                                             ; preds = %.preheader1084, %1286
  %1284 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39173
  %1285 = icmp slt i64 %1284, 0
  br i1 %1285, label %1286, label %__rustc::__rust_alloc (.exit87)

1286:                                             ; preds = %1283
  %1287 = add nsw i64 %1284, 1
  %1288 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1284, i64 %1287 acq_rel acquire, align 8, !noalias !39173
  %1289 = extractvalue { i64, i1 } %1288, 1
  br i1 %1289, label %1290, label %1283

1290:                                             ; preds = %1286
  %1291 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !39173
  %1292 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 72 monotonic, align 8, !noalias !39173
  %1293 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 72 monotonic, align 8, !noalias !39173
  %1294 = call i64 @llvm.sadd.sat.i64(i64 %1293, i64 72)
  %1295 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !39173
  br label %1296

1296:                                             ; preds = %1299, %1290
  %1297 = phi i64 [ %1295, %1290 ], [ %1302, %1299 ]
  %1298 = icmp sgt i64 %1294, %1297
  br i1 %1298, label %1299, label %1303

1299:                                             ; preds = %1296
  %1300 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %1297, i64 %1294 monotonic monotonic, align 8, !noalias !39173
  %1301 = extractvalue { i64, i1 } %1300, 1
  %1302 = extractvalue { i64, i1 } %1300, 0
  br i1 %1301, label %1303, label %1296

1303:                                             ; preds = %1299, %1296
  %1304 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39173
  br label %__rustc::__rust_alloc (.exit87)

__rustc::__rust_alloc (.exit87.thread): ; preds = %1264
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef 72) #94
          to label %1305 unwind label %1306, !noalias !38924

1305:                                             ; preds = %__rustc::__rust_alloc (.exit87.thread)
  unreachable

1306:                                             ; preds = %__rustc::__rust_alloc (.exit87.thread)
  %1307 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef readonly align 8 dereferenceable(56) %1265)
          to label %1445 unwind label %1308, !noalias !38924

1308:                                             ; preds = %1306
  %1309 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924
  unreachable

1310:                                             ; preds = %1261, %1257
  %1311 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(56) %13) #90
          to label %1445 unwind label %1312, !noalias !38924

1312:                                             ; preds = %1310, %1253
  %1313 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !38924
  unreachable

__rustc::__rust_alloc (.exit87): ; preds = %1283, %1303
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %1267, ptr noundef nonnull align 8 dereferenceable(72) %11, i64 72, i1 false), !noalias !38924
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !38924
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !38924
  br label %963

1314:                                             ; preds = %1016, %1013, %1009
  call void @llvm.lifetime.end.p0(ptr nonnull %31), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %32), !noalias !38952
  %1315 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1315, ptr noundef nonnull align 8 dereferenceable(96) %33, i64 96, i1 false), !noalias !38996
  store i64 0, ptr %0, align 16, !alias.scope !38893, !noalias !38996
  call void @llvm.lifetime.end.p0(ptr nonnull %33), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !38952
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !38952
  br label %1460

1316:                                             ; preds = %852
  call void @llvm.lifetime.end.p0(ptr nonnull %45), !noalias !38952
  store i32 %249, ptr %248, align 16, !alias.scope !38898, !noalias !38901
  store i32 %251, ptr %250, align 4, !alias.scope !38898, !noalias !38901
  call void @llvm.lifetime.start.p0(ptr nonnull %44), !noalias !38952
; invoke <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %44, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %38)
          to label %1317 unwind label %799

1317:                                             ; preds = %1316
  %1318 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1318, ptr noundef nonnull align 8 dereferenceable(96) %44, i64 96, i1 false), !noalias !38996
  store i64 0, ptr %0, align 16, !alias.scope !38893, !noalias !38996
  call void @llvm.lifetime.end.p0(ptr nonnull %44), !noalias !38952
  br label %1319

1319:                                             ; preds = %1317, %389
  %1320 = phi i8 [ 1, %389 ], [ 0, %1317 ]
  %1321 = icmp eq i64 %243, 0
  br i1 %1321, label %1023, label %1322

1322:                                             ; preds = %1319
  %1323 = shl nuw i64 %243, 2
  %1324 = load i64, ptr %287, align 8, !noalias !39176, !noundef !1740
  %1325 = call i64 @llvm.umin.i64(i64 %1323, i64 9223372036854775807)
  %1326 = call i64 @llvm.ssub.sat.i64(i64 %1324, i64 %1325)
  store i64 %1326, ptr %287, align 8, !noalias !39176
  %1327 = load i64, ptr %288, align 8, !noalias !39176, !noundef !1740
  %1328 = icmp slt i64 %1326, %1327
  br i1 %1328, label %1329, label %.preheader1088

1329:                                             ; preds = %1322
  store i64 %1326, ptr %288, align 8, !noalias !39176
  br label %.preheader1088

.preheader1088:                                   ; preds = %1329, %1322
  br label %1330

1330:                                             ; preds = %.preheader1088, %1333
  %1331 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39176
  %1332 = icmp slt i64 %1331, 0
  br i1 %1332, label %1333, label %__rustc::__rust_dealloc (.exit88)

1333:                                             ; preds = %1330
  %1334 = add nsw i64 %1331, 1
  %1335 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1331, i64 %1334 acq_rel acquire, align 8, !noalias !39176
  %1336 = extractvalue { i64, i1 } %1335, 1
  br i1 %1336, label %1337, label %1330

1337:                                             ; preds = %1333
  %1338 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1325 monotonic, align 8, !noalias !39176
  %1339 = call i64 @llvm.ssub.sat.i64(i64 %1338, i64 %1325)
  %1340 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39176
  br label %1341

1341:                                             ; preds = %1344, %1337
  %1342 = phi i64 [ %1340, %1337 ], [ %1347, %1344 ]
  %1343 = icmp slt i64 %1339, %1342
  br i1 %1343, label %1344, label %1348

1344:                                             ; preds = %1341
  %1345 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1342, i64 %1339 monotonic monotonic, align 8, !noalias !39176
  %1346 = extractvalue { i64, i1 } %1345, 1
  %1347 = extractvalue { i64, i1 } %1345, 0
  br i1 %1346, label %1348, label %1341

1348:                                             ; preds = %1344, %1341
  %1349 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39176
  br label %__rustc::__rust_dealloc (.exit88)

__rustc::__rust_dealloc (.exit88): ; preds = %1330, %1348
  call void @free(ptr noundef nonnull %242) #93, !noalias !39176
  br label %1023

1350:                                             ; preds = %790
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef align 8 dereferenceable(96) %261) #90
          to label %294 unwind label %1019, !inline_history !38900

1351:                                             ; preds = %1445, %860
  %1352 = phi i64 [ %863, %860 ], [ %1449, %1445 ]
  %1353 = phi i8 [ 0, %860 ], [ %1448, %1445 ]
  %1354 = phi i1 [ %864, %860 ], [ %1447, %1445 ]
  %1355 = phi { ptr, i32 } [ %861, %860 ], [ %1446, %1445 ]
  %1356 = icmp ne i64 %1352, -1
  %1357 = and i1 %1356, %1354
  br i1 %1357, label %1450, label %64

1358:                                             ; preds = %__rustc::__rust_dealloc (.exit84), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %48), !noalias !38952
  %1359 = load i64, ptr %49, align 8, !range !2059, !noalias !38952, !noundef !1740
  %1360 = icmp eq i64 %1359, -1
  br i1 %1360, label %1361, label %1363

1361:                                             ; preds = %1363, %1358
  call void @llvm.lifetime.end.p0(ptr nonnull %49), !noalias !38952
  %1362 = trunc nuw i8 %1025 to i1
  br i1 %1362, label %1364, label %1460

1363:                                             ; preds = %1358
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(56) %49)
          to label %1361 unwind label %245, !noalias !38924, !inline_history !38900

1364:                                             ; preds = %1361
  %1365 = getelementptr inbounds nuw i8, ptr %38, i64 72
  %1366 = load i64, ptr %1365, align 8, !range !1778, !noundef !1740
  %1367 = icmp ugt i64 %1366, 5
  br i1 %1367, label %1368, label %1402

1368:                                             ; preds = %1364
  %1369 = getelementptr inbounds nuw i8, ptr %38, i64 80
  %1370 = load ptr, ptr %1369, align 8, !nonnull !1740, !noundef !1740
  %1371 = mul i64 %1366, 3
  %1372 = add i64 %1371, -3
  %1373 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1374 = load i64, ptr %1373, align 8, !noalias !39179, !noundef !1740
  %1375 = call i64 @llvm.umin.i64(i64 %1372, i64 9223372036854775807)
  %1376 = call i64 @llvm.ssub.sat.i64(i64 %1374, i64 %1375)
  store i64 %1376, ptr %1373, align 8, !noalias !39179
  %1377 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1378 = load i64, ptr %1377, align 8, !noalias !39179, !noundef !1740
  %1379 = icmp slt i64 %1376, %1378
  br i1 %1379, label %1380, label %.preheader1081

1380:                                             ; preds = %1368
  store i64 %1376, ptr %1377, align 8, !noalias !39179
  br label %.preheader1081

.preheader1081:                                   ; preds = %1380, %1368
  br label %1381

1381:                                             ; preds = %.preheader1081, %1384
  %1382 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39179
  %1383 = icmp slt i64 %1382, 0
  br i1 %1383, label %1384, label %__rustc::__rust_dealloc (.exit89)

1384:                                             ; preds = %1381
  %1385 = add nsw i64 %1382, 1
  %1386 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1382, i64 %1385 acq_rel acquire, align 8, !noalias !39179
  %1387 = extractvalue { i64, i1 } %1386, 1
  br i1 %1387, label %1388, label %1381

1388:                                             ; preds = %1384
  %1389 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1375 monotonic, align 8, !noalias !39179
  %1390 = call i64 @llvm.ssub.sat.i64(i64 %1389, i64 %1375)
  %1391 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39179
  br label %1392

1392:                                             ; preds = %1395, %1388
  %1393 = phi i64 [ %1391, %1388 ], [ %1398, %1395 ]
  %1394 = icmp slt i64 %1390, %1393
  br i1 %1394, label %1395, label %1399

1395:                                             ; preds = %1392
  %1396 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1393, i64 %1390 monotonic monotonic, align 8, !noalias !39179
  %1397 = extractvalue { i64, i1 } %1396, 1
  %1398 = extractvalue { i64, i1 } %1396, 0
  br i1 %1397, label %1399, label %1392

1399:                                             ; preds = %1395, %1392
  %1400 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39179
  br label %__rustc::__rust_dealloc (.exit89)

__rustc::__rust_dealloc (.exit89): ; preds = %1381, %1399
  %1401 = icmp ne i64 %1372, 0
  call void @llvm.assume(i1 %1401), !noalias !39179
  call void @free(ptr noundef nonnull %1370) #93, !noalias !39179
  br label %1402

1402:                                             ; preds = %__rustc::__rust_dealloc (.exit89), %1364
  %1403 = load i64, ptr %38, align 8, !range !2059, !noundef !1740
  %1404 = icmp sgt i64 %1403, 0
  br i1 %1404, label %1405, label %1437

1405:                                             ; preds = %1402
  %1406 = getelementptr inbounds nuw i8, ptr %38, i64 8
  %1407 = load ptr, ptr %1406, align 8, !nonnull !1740, !noundef !1740
  %1408 = mul nuw i64 %1403, 3
  %1409 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1410 = load i64, ptr %1409, align 8, !noalias !39184, !noundef !1740
  %1411 = call i64 @llvm.umin.i64(i64 %1408, i64 9223372036854775807)
  %1412 = call i64 @llvm.ssub.sat.i64(i64 %1410, i64 %1411)
  store i64 %1412, ptr %1409, align 8, !noalias !39184
  %1413 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1414 = load i64, ptr %1413, align 8, !noalias !39184, !noundef !1740
  %1415 = icmp slt i64 %1412, %1414
  br i1 %1415, label %1416, label %.preheader1080

1416:                                             ; preds = %1405
  store i64 %1412, ptr %1413, align 8, !noalias !39184
  br label %.preheader1080

.preheader1080:                                   ; preds = %1416, %1405
  br label %1417

1417:                                             ; preds = %.preheader1080, %1420
  %1418 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39184
  %1419 = icmp slt i64 %1418, 0
  br i1 %1419, label %1420, label %__rustc::__rust_dealloc (.exit90)

1420:                                             ; preds = %1417
  %1421 = add nsw i64 %1418, 1
  %1422 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1418, i64 %1421 acq_rel acquire, align 8, !noalias !39184
  %1423 = extractvalue { i64, i1 } %1422, 1
  br i1 %1423, label %1424, label %1417

1424:                                             ; preds = %1420
  %1425 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1411 monotonic, align 8, !noalias !39184
  %1426 = call i64 @llvm.ssub.sat.i64(i64 %1425, i64 %1411)
  %1427 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39184
  br label %1428

1428:                                             ; preds = %1431, %1424
  %1429 = phi i64 [ %1427, %1424 ], [ %1434, %1431 ]
  %1430 = icmp slt i64 %1426, %1429
  br i1 %1430, label %1431, label %1435

1431:                                             ; preds = %1428
  %1432 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1429, i64 %1426 monotonic monotonic, align 8, !noalias !39184
  %1433 = extractvalue { i64, i1 } %1432, 1
  %1434 = extractvalue { i64, i1 } %1432, 0
  br i1 %1433, label %1435, label %1428

1435:                                             ; preds = %1431, %1428
  %1436 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39184
  br label %__rustc::__rust_dealloc (.exit90)

__rustc::__rust_dealloc (.exit90): ; preds = %1417, %1435
  call void @free(ptr noundef nonnull %1407) #93, !noalias !39184
  br label %1437

1437:                                             ; preds = %__rustc::__rust_dealloc (.exit90), %1402
  %1438 = getelementptr inbounds nuw i8, ptr %38, i64 96
  %1439 = load ptr, ptr %1438, align 8, !noundef !1740
  %1440 = icmp eq ptr %1439, null
  br i1 %1440, label %1460, label %1441

1441:                                             ; preds = %1437
  %1442 = atomicrmw sub ptr %1439, i64 1 release, align 8, !noalias !39185
  %1443 = icmp eq i64 %1442, 1
  br i1 %1443, label %1444, label %1460

1444:                                             ; preds = %1441
  fence acquire, !noalias !38924
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1438) #92
  br label %1460

1445:                                             ; preds = %1310, %1306, %1253, %1249, %1179, %1175, %1168, %1164, %1108, %1104, %940, %907, %857, %298, %294
  %1446 = phi { ptr, i32 } [ %1307, %1306 ], [ %859, %857 ], [ %908, %907 ], [ %1176, %1175 ], [ %1176, %1179 ], [ %1311, %1310 ], [ %1250, %1249 ], [ %1250, %1253 ], [ %296, %298 ], [ %296, %294 ], [ %941, %940 ], [ %1105, %1104 ], [ %1169, %1168 ], [ %1105, %1108 ], [ %1165, %1164 ]
  %1447 = phi i1 [ true, %1306 ], [ true, %857 ], [ false, %907 ], [ true, %1175 ], [ true, %1179 ], [ true, %1310 ], [ true, %1249 ], [ true, %1253 ], [ true, %298 ], [ true, %294 ], [ true, %940 ], [ true, %1104 ], [ true, %1168 ], [ true, %1108 ], [ true, %1164 ]
  %1448 = phi i8 [ 1, %1306 ], [ %858, %857 ], [ 1, %907 ], [ 1, %1175 ], [ 1, %1179 ], [ 1, %1310 ], [ 1, %1249 ], [ 1, %1253 ], [ %295, %298 ], [ %295, %294 ], [ 1, %940 ], [ 1, %1104 ], [ 1, %1168 ], [ 1, %1108 ], [ 1, %1164 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %48) #90, !noalias !38924, !inline_history !38900
  %1449 = load i64, ptr %49, align 8, !range !2059, !noalias !38952
  br label %1351

1450:                                             ; preds = %1351
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::VarSchema> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(56) %49) #90
          to label %64 unwind label %1019, !noalias !38924, !inline_history !38900

1451:                                             ; preds = %.loopexit112
  %1452 = landingpad { ptr, i32 }
          cleanup
  %1453 = icmp eq i64 %243, 0
  br i1 %1453, label %1458, label %1454

1454:                                             ; preds = %1451
  %1455 = shl nuw i64 %243, 2
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %242) ], !noalias !38924
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %242, i64 noundef %1455, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !38924
  br label %1458

1456:                                             ; preds = %1800, %1797, %1793, %1512, %1458, %64
  %1457 = phi { ptr, i32 } [ %66, %64 ], [ %1459, %1458 ], [ %1523, %1512 ], [ %1720, %1800 ], [ %1720, %1793 ], [ %1720, %1797 ]
  resume { ptr, i32 } %1457

1458:                                             ; preds = %1454, %1451, %235, %231, %68, %64
  %1459 = phi { ptr, i32 } [ %66, %64 ], [ %69, %68 ], [ %232, %235 ], [ %232, %231 ], [ %1452, %1451 ], [ %1452, %1454 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %38) #90
          to label %1456 unwind label %1019, !inline_history !38900

1460:                                             ; preds = %1444, %1441, %1437, %1361, %1314
  call void @llvm.lifetime.end.p0(ptr nonnull %47)
  br label %1690

1461:                                             ; preds = %5
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %58, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  %1462 = getelementptr inbounds nuw i8, ptr %4, i64 664
  %1463 = load ptr, ptr %1462, align 8, !nonnull !1740, !align !1836, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %60)
  %1464 = load ptr, ptr %63, align 8, !nonnull !1740, !noundef !1740
  %1465 = getelementptr inbounds nuw i8, ptr %2, i64 16
  %1466 = load i64, ptr %1465, align 8, !noundef !1740
  tail call void @llvm.experimental.noalias.scope.decl(metadata !39192)
  %1467 = getelementptr inbounds nuw i8, ptr %1464, i64 16
  %1468 = icmp slt i64 %1466, 0
  br i1 %1468, label %__rustc::__rust_alloc (.exit91.thread), label %1469, !prof !6191

1469:                                             ; preds = %1461
  %1470 = icmp eq i64 %1466, 0
  br i1 %1470, label %1515, label %1471

1471:                                             ; preds = %1469
  %1472 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %1466) #93, !noalias !39195
  %1473 = icmp eq ptr %1472, null
  br i1 %1473, label %__rustc::__rust_alloc (.exit91.thread), label %1474

1474:                                             ; preds = %1471
  %1475 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1476 = load i64, ptr %1475, align 8, !noalias !39195, !noundef !1740
  %1477 = tail call i64 @llvm.uadd.sat.i64(i64 %1476, i64 1)
  store i64 %1477, ptr %1475, align 8, !noalias !39195
  %1478 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1479 = load i64, ptr %1478, align 8, !noalias !39195, !noundef !1740
  %1480 = tail call i64 @llvm.uadd.sat.i64(i64 %1479, i64 %1466)
  store i64 %1480, ptr %1478, align 8, !noalias !39195
  %1481 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1482 = load i64, ptr %1481, align 8, !noalias !39195, !noundef !1740
  %1483 = tail call i64 @llvm.sadd.sat.i64(i64 %1482, i64 %1466)
  store i64 %1483, ptr %1481, align 8, !noalias !39195
  %1484 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1485 = load i64, ptr %1484, align 8, !noalias !39195, !noundef !1740
  %1486 = icmp sgt i64 %1483, %1485
  br i1 %1486, label %1487, label %.preheader1276

1487:                                             ; preds = %1474
  store i64 %1483, ptr %1484, align 8, !noalias !39195
  br label %.preheader1276

.preheader1276:                                   ; preds = %1487, %1474
  br label %1488

1488:                                             ; preds = %.preheader1276, %1491
  %1489 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39195
  %1490 = icmp slt i64 %1489, 0
  br i1 %1490, label %1491, label %__rustc::__rust_alloc (.exit91)

1491:                                             ; preds = %1488
  %1492 = add nsw i64 %1489, 1
  %1493 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1489, i64 %1492 acq_rel acquire, align 8, !noalias !39195
  %1494 = extractvalue { i64, i1 } %1493, 1
  br i1 %1494, label %1495, label %1488

1495:                                             ; preds = %1491
  %1496 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !39195
  %1497 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %1466 monotonic, align 8, !noalias !39195
  %1498 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1466 monotonic, align 8, !noalias !39195
  %1499 = tail call i64 @llvm.sadd.sat.i64(i64 %1498, i64 %1466)
  %1500 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !39195
  br label %1501

1501:                                             ; preds = %1504, %1495
  %1502 = phi i64 [ %1500, %1495 ], [ %1507, %1504 ]
  %1503 = icmp sgt i64 %1499, %1502
  br i1 %1503, label %1504, label %1508

1504:                                             ; preds = %1501
  %1505 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %1502, i64 %1499 monotonic monotonic, align 8, !noalias !39195
  %1506 = extractvalue { i64, i1 } %1505, 1
  %1507 = extractvalue { i64, i1 } %1505, 0
  br i1 %1506, label %1508, label %1501

1508:                                             ; preds = %1504, %1501
  %1509 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39195
  br label %__rustc::__rust_alloc (.exit91)

__rustc::__rust_alloc (.exit91.thread): ; preds = %1471, %1461
  %1510 = phi i64 [ 1, %1471 ], [ 0, %1461 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %1510, i64 %1466) #94
          to label %1511 unwind label %1513

1511:                                             ; preds = %__rustc::__rust_alloc (.exit91.thread)
  unreachable

__rustc::__rust_alloc (.exit91): ; preds = %1488, %1508
  tail call void @llvm.memcpy.p0.p0.i64(ptr nonnull align 1 %1472, ptr nonnull readonly align 1 %1467, i64 %1466, i1 false), !noalias !39192
  br label %1515

1512:                                             ; preds = %1521
  br i1 %1522, label %1719, label %1456

1513:                                             ; preds = %1594, %__rustc::__rust_alloc (.exit91.thread)
  %1514 = landingpad { ptr, i32 }
          cleanup
  br label %1719

1515:                                             ; preds = %__rustc::__rust_alloc (.exit91), %1469
  %1516 = phi ptr [ %1472, %__rustc::__rust_alloc (.exit91) ], [ inttoptr (i64 1 to ptr), %1469 ]
  %1517 = getelementptr inbounds nuw i8, ptr %60, i64 8
  store i64 %1466, ptr %1517, align 8, !alias.scope !39192
  %1518 = getelementptr inbounds nuw i8, ptr %60, i64 16
  store ptr %1516, ptr %1518, align 8, !alias.scope !39192
  %1519 = getelementptr inbounds nuw i8, ptr %60, i64 24
  store i64 %1466, ptr %1519, align 8, !alias.scope !39192
  store i64 -9223372036854775808, ptr %60, align 8, !alias.scope !39192
; invoke <purrdf_core::ir::dataset::RdfDataset>::term_id_by_value
  %1520 = invoke noundef i32 @<purrdf_core::ir::dataset::RdfDataset>::term_id_by_value(ptr noundef nonnull align 8 %1463, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %60)
          to label %1527 unwind label %1524

1521:                                             ; preds = %1691, %1631, %1524
  %1522 = phi i1 [ %1525, %1524 ], [ false, %1691 ], [ false, %1631 ]
  %1523 = phi { ptr, i32 } [ %1526, %1524 ], [ %1692, %1691 ], [ %1632, %1631 ]
; invoke core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(80) %60) #90
          to label %1512 unwind label %1693

1524:                                             ; preds = %1625, %1624, %1597, %.loopexit1274, %1515
  %1525 = phi i1 [ false, %1624 ], [ false, %1625 ], [ true, %1597 ], [ true, %.loopexit1274 ], [ true, %1515 ]
  %1526 = landingpad { ptr, i32 }
          cleanup
  br label %1521

1527:                                             ; preds = %1515
  %1528 = icmp eq i32 %1520, 0
  br i1 %1528, label %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread), label %1529

1529:                                             ; preds = %1527
  %1530 = load ptr, ptr %1462, align 8, !nonnull !1740, !align !1836, !noundef !1740
  %1531 = getelementptr inbounds nuw i8, ptr %1530, i64 152
  %1532 = load ptr, ptr %1531, align 8, !nonnull !1740, !noundef !1740
  %1533 = getelementptr inbounds nuw i8, ptr %1530, i64 160
  %1534 = load i64, ptr %1533, align 8, !noundef !1740
  switch i64 %1534, label %.preheader.i [
    i64 0, label %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread)
    i64 1, label %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit)
  ]

.preheader.i:                                     ; preds = %1529, %.preheader.i
  %1535 = phi i64 [ %1544, %.preheader.i ], [ %1534, %1529 ]
  %1536 = phi i64 [ %1543, %.preheader.i ], [ 0, %1529 ]
  %1537 = lshr i64 %1535, 1
  %1538 = add nuw i64 %1537, %1536
  %1539 = icmp ult i64 %1538, %1534
  call void @llvm.assume(i1 %1539)
  %1540 = getelementptr inbounds nuw [4 x i8], ptr %1532, i64 %1538
  %1541 = load i32, ptr %1540, align 4, !range !3837, !alias.scope !39198, !noundef !1740
  %1542 = icmp ugt i32 %1541, %1520
  %1543 = select i1 %1542, i64 %1536, i64 %1538, !unpredictable !1740
  %1544 = sub i64 %1535, %1537
  %1545 = icmp ugt i64 %1544, 1
  br i1 %1545, label %.preheader.i, label %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit)

<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit): ; preds = %.preheader.i, %1529
  %1546 = phi i64 [ 0, %1529 ], [ %1543, %.preheader.i ]
  %1547 = getelementptr inbounds nuw [4 x i8], ptr %1532, i64 %1546
  %1548 = load i32, ptr %1547, align 4, !range !3837, !alias.scope !39198, !noundef !1740
  %1549 = icmp eq i32 %1548, %1520
  br i1 %1549, label %1551, label %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread)

<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread): ; preds = %._crit_edge, %1529, %1555, %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit), %1527
  call void @llvm.lifetime.start.p0(ptr nonnull %53)
  call void @llvm.lifetime.start.p0(ptr nonnull %52)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %52, ptr noundef nonnull align 8 dereferenceable(104) %58, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %51)
; invoke purrdf_sparql_eval::eval::syntactic_schema
  %1550 = invoke noundef nonnull ptr @purrdf_sparql_eval::eval::syntactic_schema(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3)
          to label %1633 unwind label %1691

1551:                                             ; preds = %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit)
  %1552 = getelementptr inbounds nuw i8, ptr %4, i64 1144
  call void @llvm.experimental.noalias.scope.decl(metadata !39201)
  %1553 = load i64, ptr %1552, align 8, !range !1739, !alias.scope !39201, !noundef !1740
  %1554 = trunc nuw i64 %1553 to i1
  br i1 %1554, label %1555, label %.loopexit1274

1555:                                             ; preds = %1551
  %1556 = getelementptr inbounds nuw i8, ptr %4, i64 1152
  %1557 = load ptr, ptr %1556, align 16, !alias.scope !39201, !noundef !1740
  %1558 = icmp eq ptr %1557, null
  br i1 %1558, label %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread), label %1559

1559:                                             ; preds = %1555
  %1560 = getelementptr inbounds nuw i8, ptr %4, i64 1160
  %1561 = load i64, ptr %1560, align 8, !alias.scope !39201
  br label %1562

1562:                                             ; preds = %1582, %1559
  %1563 = phi i64 [ %1587, %1582 ], [ %1561, %1559 ]
  %1564 = phi ptr [ %1586, %1582 ], [ %1557, %1559 ]
  %1565 = getelementptr inbounds nuw i8, ptr %1564, i64 8
  %1566 = getelementptr inbounds nuw i8, ptr %1564, i64 54
  %1567 = load i16, ptr %1566, align 2, !noalias !39204, !noundef !1740
  %1568 = zext i16 %1567 to i64
  %.idx = shl nuw nsw i64 %1568, 2
  %1569 = getelementptr inbounds nuw i8, ptr %1565, i64 %.idx
  %1570 = icmp eq i16 %1567, 0
  br i1 %1570, label %._crit_edge, label %.lr.ph

1571:                                             ; preds = %.lr.ph
  %1572 = getelementptr inbounds nuw i8, ptr %1575, i64 4
  %1573 = add nuw nsw i64 %1576, 1
  %1574 = icmp eq ptr %1572, %1569
  br i1 %1574, label %._crit_edge, label %.lr.ph

.lr.ph:                                           ; preds = %1562, %1571
  %1575 = phi ptr [ %1572, %1571 ], [ %1565, %1562 ]
  %1576 = phi i64 [ %1573, %1571 ], [ 0, %1562 ]
  %1577 = load i32, ptr %1575, align 4, !range !3837, !noalias !39204, !noundef !1740
  %1578 = call noundef range(i8 -1, 2) i8 @llvm.ucmp.i8.i32(i32 range(i32 1, 0) %1520, i32 %1577)
  switch i8 %1578, label %1579 [
    i8 -1, label %._crit_edge
    i8 0, label %.loopexit1274
    i8 1, label %1571
  ]

1579:                                             ; preds = %.lr.ph
  unreachable

._crit_edge:                                      ; preds = %1571, %.lr.ph, %1562
  %1580 = phi i64 [ %1568, %1562 ], [ %1568, %1571 ], [ %1576, %.lr.ph ]
  %1581 = icmp eq i64 %1563, 0
  br i1 %1581, label %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread), label %1582

1582:                                             ; preds = %._crit_edge
  %1583 = getelementptr inbounds nuw i8, ptr %1564, i64 56
  %1584 = icmp samesign ult i64 %1580, 12
  call void @llvm.assume(i1 %1584)
  %1585 = getelementptr inbounds nuw [8 x i8], ptr %1583, i64 %1580
  %1586 = load ptr, ptr %1585, align 8, !noalias !39204, !nonnull !1740, !noundef !1740
  %1587 = add i64 %1563, -1
  br label %1562

.loopexit1274:                                    ; preds = %.lr.ph, %1551
  %1588 = getelementptr inbounds nuw i8, ptr %4, i64 784
  %1589 = getelementptr inbounds nuw i8, ptr %4, i64 788
  %1590 = load <2 x i32>, ptr %1588, align 16
  store i32 2, ptr %1588, align 16
  store i32 %1520, ptr %1589, align 4
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %50, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %3, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %4, ptr noundef nonnull readonly align 8 dereferenceable(144) %3)
          to label %1591 unwind label %1524, !inline_history !13414

1591:                                             ; preds = %.loopexit1274
  store <2 x i32> %1590, ptr %1588, align 16
  call void @llvm.lifetime.start.p0(ptr nonnull %56)
  %1592 = load i64, ptr %50, align 16, !range !1739, !noundef !1740
  %1593 = trunc nuw i64 %1592 to i1
  br i1 %1593, label %1594, label %1597

1594:                                             ; preds = %1591
  %1595 = getelementptr inbounds nuw i8, ptr %50, i64 16
  %1596 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %1596, ptr noundef nonnull align 16 dereferenceable(96) %1595, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
; invoke core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(80) %60)
          to label %1695 unwind label %1513

1597:                                             ; preds = %1591
  %1598 = getelementptr inbounds nuw i8, ptr %50, i64 8
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %56, ptr noalias nofree noundef align 8 dereferenceable(104) %58, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %1598)
          to label %1599 unwind label %1524

1599:                                             ; preds = %1597
  %1600 = load i64, ptr %56, align 8, !range !2059, !noundef !1740
  %1601 = icmp eq i64 %1600, -1
  br i1 %1601, label %1625, label %1602

1602:                                             ; preds = %1599
  call void @llvm.lifetime.start.p0(ptr nonnull %57)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %57, ptr noundef nonnull align 8 dereferenceable(32) %56, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  call void @llvm.lifetime.start.p0(ptr nonnull %55)
  call void @llvm.lifetime.start.p0(ptr nonnull %54)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %54, ptr noundef nonnull align 8 dereferenceable(104) %58, i64 104, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !39207)
  call void @llvm.experimental.noalias.scope.decl(metadata !39210)
  call void @llvm.experimental.noalias.scope.decl(metadata !39212)
  %1603 = load i64, ptr %54, align 8, !range !2059, !alias.scope !39210, !noalias !39214, !noundef !1740
  %1604 = icmp eq i64 %1603, -1
  br i1 %1604, label %1606, label %1605

1605:                                             ; preds = %1602
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %55, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %57, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %58)
  br label %1608

1606:                                             ; preds = %1602
  %1607 = getelementptr inbounds nuw i8, ptr %55, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %1607, ptr noundef nonnull readonly align 8 dereferenceable(32) %57, i64 32, i1 false), !alias.scope !39214, !noalias !39210
  store i64 -1, ptr %55, align 8, !alias.scope !39207, !noalias !39215
  br label %1608

1608:                                             ; preds = %1606, %1605
  %1609 = getelementptr inbounds nuw i8, ptr %54, i64 72
  %1610 = load i64, ptr %1609, align 8, !range !1778, !alias.scope !39216, !noalias !39214, !noundef !1740
  %1611 = icmp ugt i64 %1610, 5
  br i1 %1611, label %1612, label %1617

1612:                                             ; preds = %1608
  %1613 = getelementptr inbounds nuw i8, ptr %54, i64 80
  %1614 = load ptr, ptr %1613, align 8, !alias.scope !39210, !noalias !39214, !nonnull !1740, !noundef !1740
  %1615 = mul i64 %1610, 3
  %1616 = add i64 %1615, -3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1614, i64 noundef %1616, i64 noundef range(i64 1, -9223372036854775807) 1) #93, !noalias !39219
  br label %1617

1617:                                             ; preds = %1612, %1608
  %1618 = getelementptr inbounds nuw i8, ptr %54, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !39222)
  %1619 = load ptr, ptr %1618, align 8, !alias.scope !39225, !noalias !39214, !noundef !1740
  %1620 = icmp eq ptr %1619, null
  br i1 %1620, label %1626, label %1621

1621:                                             ; preds = %1617
  %1622 = atomicrmw sub ptr %1619, i64 1 release, align 8, !noalias !39226
  %1623 = icmp eq i64 %1622, 1
  br i1 %1623, label %1624, label %1626

1624:                                             ; preds = %1621
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1618) #92
          to label %1626 unwind label %1524

1625:                                             ; preds = %1599
  call void @llvm.lifetime.end.p0(ptr nonnull %56)
  call void @llvm.lifetime.start.p0(ptr nonnull %59)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %59, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %58)
          to label %1629 unwind label %1524

1626:                                             ; preds = %1624, %1621, %1617
  call void @llvm.lifetime.end.p0(ptr nonnull %54)
  %1627 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1627, ptr noundef nonnull align 8 dereferenceable(96) %55, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %55)
  call void @llvm.lifetime.end.p0(ptr nonnull %57)
  br label %1628

1628:                                             ; preds = %1688, %1626
; call core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
  call fastcc void @core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(80) %60)
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  br label %1690

1629:                                             ; preds = %1625
  %1630 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1630, ptr noundef nonnull align 8 dereferenceable(96) %59, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %59)
; call core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
  call fastcc void @core::ptr::drop_glue::<purrdf_core::ir::term::TermValue> (.llvm.11631829254914579133)(ptr noalias nofree noundef align 8 dereferenceable(80) %60)
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  br label %1690

1631:                                             ; preds = %1687
  %1632 = landingpad { ptr, i32 }
          cleanup
  br label %1521

1633:                                             ; preds = %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread)
  %1634 = getelementptr inbounds nuw i8, ptr %51, i64 24
  store ptr %1550, ptr %1634, align 8, !alias.scope !39231
  store i64 0, ptr %51, align 8, !alias.scope !39231
  %1635 = getelementptr inbounds nuw i8, ptr %51, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %1635, align 8, !alias.scope !39231
  %1636 = getelementptr inbounds nuw i8, ptr %51, i64 16
  store i64 0, ptr %1636, align 8, !alias.scope !39231
  call void @llvm.experimental.noalias.scope.decl(metadata !39234)
  call void @llvm.experimental.noalias.scope.decl(metadata !39237)
  call void @llvm.experimental.noalias.scope.decl(metadata !39239)
  %1637 = load i64, ptr %52, align 8, !range !2059, !alias.scope !39237, !noalias !39241, !noundef !1740
  %1638 = icmp eq i64 %1637, -1
  br i1 %1638, label %1640, label %1639

1639:                                             ; preds = %1633
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %53, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %51, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %58)
  br label %1642

1640:                                             ; preds = %1633
  %1641 = getelementptr inbounds nuw i8, ptr %53, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %1641, ptr noundef nonnull readonly align 8 dereferenceable(32) %51, i64 32, i1 false), !alias.scope !39241, !noalias !39237
  store i64 -1, ptr %53, align 8, !alias.scope !39234, !noalias !39242
  br label %1642

1642:                                             ; preds = %1640, %1639
  %1643 = getelementptr inbounds nuw i8, ptr %52, i64 72
  %1644 = load i64, ptr %1643, align 8, !range !1778, !alias.scope !39243, !noalias !39241, !noundef !1740
  %1645 = icmp ugt i64 %1644, 5
  br i1 %1645, label %1646, label %1680

1646:                                             ; preds = %1642
  %1647 = getelementptr inbounds nuw i8, ptr %52, i64 80
  %1648 = load ptr, ptr %1647, align 8, !alias.scope !39237, !noalias !39241, !nonnull !1740, !noundef !1740
  %1649 = mul i64 %1644, 3
  %1650 = add i64 %1649, -3
  %1651 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1652 = load i64, ptr %1651, align 8, !noalias !39246, !noundef !1740
  %1653 = call i64 @llvm.umin.i64(i64 %1650, i64 9223372036854775807)
  %1654 = call i64 @llvm.ssub.sat.i64(i64 %1652, i64 %1653)
  store i64 %1654, ptr %1651, align 8, !noalias !39246
  %1655 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1656 = load i64, ptr %1655, align 8, !noalias !39246, !noundef !1740
  %1657 = icmp slt i64 %1654, %1656
  br i1 %1657, label %1658, label %.preheader1272

1658:                                             ; preds = %1646
  store i64 %1654, ptr %1655, align 8, !noalias !39246
  br label %.preheader1272

.preheader1272:                                   ; preds = %1658, %1646
  br label %1659

1659:                                             ; preds = %.preheader1272, %1662
  %1660 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39246
  %1661 = icmp slt i64 %1660, 0
  br i1 %1661, label %1662, label %__rustc::__rust_dealloc (.exit92)

1662:                                             ; preds = %1659
  %1663 = add nsw i64 %1660, 1
  %1664 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1660, i64 %1663 acq_rel acquire, align 8, !noalias !39246
  %1665 = extractvalue { i64, i1 } %1664, 1
  br i1 %1665, label %1666, label %1659

1666:                                             ; preds = %1662
  %1667 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1653 monotonic, align 8, !noalias !39246
  %1668 = call i64 @llvm.ssub.sat.i64(i64 %1667, i64 %1653)
  %1669 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39246
  br label %1670

1670:                                             ; preds = %1673, %1666
  %1671 = phi i64 [ %1669, %1666 ], [ %1676, %1673 ]
  %1672 = icmp slt i64 %1668, %1671
  br i1 %1672, label %1673, label %1677

1673:                                             ; preds = %1670
  %1674 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1671, i64 %1668 monotonic monotonic, align 8, !noalias !39246
  %1675 = extractvalue { i64, i1 } %1674, 1
  %1676 = extractvalue { i64, i1 } %1674, 0
  br i1 %1675, label %1677, label %1670

1677:                                             ; preds = %1673, %1670
  %1678 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39246
  br label %__rustc::__rust_dealloc (.exit92)

__rustc::__rust_dealloc (.exit92): ; preds = %1659, %1677
  %1679 = icmp ne i64 %1650, 0
  call void @llvm.assume(i1 %1679), !noalias !39246
  call void @free(ptr noundef nonnull %1648) #93, !noalias !39246
  br label %1680

1680:                                             ; preds = %__rustc::__rust_dealloc (.exit92), %1642
  %1681 = getelementptr inbounds nuw i8, ptr %52, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !39249)
  %1682 = load ptr, ptr %1681, align 8, !alias.scope !39252, !noalias !39241, !noundef !1740
  %1683 = icmp eq ptr %1682, null
  br i1 %1683, label %1688, label %1684

1684:                                             ; preds = %1680
  %1685 = atomicrmw sub ptr %1682, i64 1 release, align 8, !noalias !39253
  %1686 = icmp eq i64 %1685, 1
  br i1 %1686, label %1687, label %1688

1687:                                             ; preds = %1684
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1681) #92
          to label %1688 unwind label %1631

1688:                                             ; preds = %1687, %1684, %1680
  call void @llvm.lifetime.end.p0(ptr nonnull %51)
  call void @llvm.lifetime.end.p0(ptr nonnull %52)
  %1689 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1689, ptr noundef nonnull align 8 dereferenceable(96) %53, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %53)
  br label %1628

1690:                                             ; preds = %1718, %1715, %1711, %1629, %1628, %1460
  ret void

1691:                                             ; preds = %<purrdf_core::ir::dataset::RdfDataset>::has_named_graph (.exit.thread)
  %1692 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %58) #90
          to label %1521 unwind label %1693

1693:                                             ; preds = %1800, %1691, %1521
  %1694 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91
  unreachable

1695:                                             ; preds = %1594
  call void @llvm.lifetime.end.p0(ptr nonnull %60)
  %1696 = getelementptr inbounds nuw i8, ptr %58, i64 72
  %1697 = load i64, ptr %1696, align 8, !range !1778, !noundef !1740
  %1698 = icmp ugt i64 %1697, 5
  br i1 %1698, label %1699, label %1704

1699:                                             ; preds = %1695
  %1700 = getelementptr inbounds nuw i8, ptr %58, i64 80
  %1701 = load ptr, ptr %1700, align 8, !nonnull !1740, !noundef !1740
  %1702 = mul i64 %1697, 3
  %1703 = add i64 %1702, -3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1701, i64 noundef %1703, i64 noundef range(i64 1, -9223372036854775807) 1) #93, !noalias !39258
  br label %1704

1704:                                             ; preds = %1699, %1695
  %1705 = load i64, ptr %58, align 8, !range !2059, !noundef !1740
  %1706 = icmp sgt i64 %1705, 0
  br i1 %1706, label %1707, label %1711

1707:                                             ; preds = %1704
  %1708 = getelementptr inbounds nuw i8, ptr %58, i64 8
  %1709 = load ptr, ptr %1708, align 8, !nonnull !1740, !noundef !1740
  %1710 = mul nuw i64 %1705, 3
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1709, i64 noundef %1710, i64 noundef range(i64 1, -9223372036854775807) 1) #93, !noalias !39263
  br label %1711

1711:                                             ; preds = %1707, %1704
  %1712 = getelementptr inbounds nuw i8, ptr %58, i64 96
  %1713 = load ptr, ptr %1712, align 8, !noundef !1740
  %1714 = icmp eq ptr %1713, null
  br i1 %1714, label %1690, label %1715

1715:                                             ; preds = %1711
  %1716 = atomicrmw sub ptr %1713, i64 1 release, align 8, !noalias !39264
  %1717 = icmp eq i64 %1716, 1
  br i1 %1717, label %1718, label %1690

1718:                                             ; preds = %1715
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1712) #92
  br label %1690

1719:                                             ; preds = %1513, %1512
  %1720 = phi { ptr, i32 } [ %1514, %1513 ], [ %1523, %1512 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !39271)
  %1721 = getelementptr inbounds nuw i8, ptr %58, i64 72
  %1722 = load i64, ptr %1721, align 8, !range !1778, !alias.scope !39274, !noundef !1740
  %1723 = icmp ugt i64 %1722, 5
  br i1 %1723, label %1724, label %1758

1724:                                             ; preds = %1719
  %1725 = getelementptr inbounds nuw i8, ptr %58, i64 80
  %1726 = load ptr, ptr %1725, align 8, !alias.scope !39271, !nonnull !1740, !noundef !1740
  %1727 = mul i64 %1722, 3
  %1728 = add i64 %1727, -3
  %1729 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1730 = load i64, ptr %1729, align 8, !noalias !39277, !noundef !1740
  %1731 = call i64 @llvm.umin.i64(i64 %1728, i64 9223372036854775807)
  %1732 = call i64 @llvm.ssub.sat.i64(i64 %1730, i64 %1731)
  store i64 %1732, ptr %1729, align 8, !noalias !39277
  %1733 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1734 = load i64, ptr %1733, align 8, !noalias !39277, !noundef !1740
  %1735 = icmp slt i64 %1732, %1734
  br i1 %1735, label %1736, label %.preheader1271

1736:                                             ; preds = %1724
  store i64 %1732, ptr %1733, align 8, !noalias !39277
  br label %.preheader1271

.preheader1271:                                   ; preds = %1736, %1724
  br label %1737

1737:                                             ; preds = %.preheader1271, %1740
  %1738 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39277
  %1739 = icmp slt i64 %1738, 0
  br i1 %1739, label %1740, label %__rustc::__rust_dealloc (.exit93)

1740:                                             ; preds = %1737
  %1741 = add nsw i64 %1738, 1
  %1742 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1738, i64 %1741 acq_rel acquire, align 8, !noalias !39277
  %1743 = extractvalue { i64, i1 } %1742, 1
  br i1 %1743, label %1744, label %1737

1744:                                             ; preds = %1740
  %1745 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1731 monotonic, align 8, !noalias !39277
  %1746 = call i64 @llvm.ssub.sat.i64(i64 %1745, i64 %1731)
  %1747 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39277
  br label %1748

1748:                                             ; preds = %1751, %1744
  %1749 = phi i64 [ %1747, %1744 ], [ %1754, %1751 ]
  %1750 = icmp slt i64 %1746, %1749
  br i1 %1750, label %1751, label %1755

1751:                                             ; preds = %1748
  %1752 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1749, i64 %1746 monotonic monotonic, align 8, !noalias !39277
  %1753 = extractvalue { i64, i1 } %1752, 1
  %1754 = extractvalue { i64, i1 } %1752, 0
  br i1 %1753, label %1755, label %1748

1755:                                             ; preds = %1751, %1748
  %1756 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39277
  br label %__rustc::__rust_dealloc (.exit93)

__rustc::__rust_dealloc (.exit93): ; preds = %1737, %1755
  %1757 = icmp ne i64 %1728, 0
  call void @llvm.assume(i1 %1757), !noalias !39277
  call void @free(ptr noundef nonnull %1726) #93, !noalias !39277
  br label %1758

1758:                                             ; preds = %__rustc::__rust_dealloc (.exit93), %1719
  %1759 = load i64, ptr %58, align 8, !range !2059, !alias.scope !39271, !noundef !1740
  %1760 = icmp sgt i64 %1759, 0
  br i1 %1760, label %1761, label %1793

1761:                                             ; preds = %1758
  %1762 = getelementptr inbounds nuw i8, ptr %58, i64 8
  %1763 = load ptr, ptr %1762, align 8, !alias.scope !39271, !nonnull !1740, !noundef !1740
  %1764 = mul nuw i64 %1759, 3
  %1765 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1766 = load i64, ptr %1765, align 8, !noalias !39271, !noundef !1740
  %1767 = call i64 @llvm.umin.i64(i64 %1764, i64 9223372036854775807)
  %1768 = call i64 @llvm.ssub.sat.i64(i64 %1766, i64 %1767)
  store i64 %1768, ptr %1765, align 8, !noalias !39271
  %1769 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %1770 = load i64, ptr %1769, align 8, !noalias !39271, !noundef !1740
  %1771 = icmp slt i64 %1768, %1770
  br i1 %1771, label %1772, label %.preheader1270

1772:                                             ; preds = %1761
  store i64 %1768, ptr %1769, align 8, !noalias !39271
  br label %.preheader1270

.preheader1270:                                   ; preds = %1772, %1761
  br label %1773

1773:                                             ; preds = %.preheader1270, %1776
  %1774 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !39271
  %1775 = icmp slt i64 %1774, 0
  br i1 %1775, label %1776, label %__rustc::__rust_dealloc (.exit94)

1776:                                             ; preds = %1773
  %1777 = add nsw i64 %1774, 1
  %1778 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1774, i64 %1777 acq_rel acquire, align 8, !noalias !39271
  %1779 = extractvalue { i64, i1 } %1778, 1
  br i1 %1779, label %1780, label %1773

1780:                                             ; preds = %1776
  %1781 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1767 monotonic, align 8, !noalias !39271
  %1782 = call i64 @llvm.ssub.sat.i64(i64 %1781, i64 %1767)
  %1783 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !39271
  br label %1784

1784:                                             ; preds = %1787, %1780
  %1785 = phi i64 [ %1783, %1780 ], [ %1790, %1787 ]
  %1786 = icmp slt i64 %1782, %1785
  br i1 %1786, label %1787, label %1791

1787:                                             ; preds = %1784
  %1788 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1785, i64 %1782 monotonic monotonic, align 8, !noalias !39271
  %1789 = extractvalue { i64, i1 } %1788, 1
  %1790 = extractvalue { i64, i1 } %1788, 0
  br i1 %1789, label %1791, label %1784

1791:                                             ; preds = %1787, %1784
  %1792 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !39271
  br label %__rustc::__rust_dealloc (.exit94)

__rustc::__rust_dealloc (.exit94): ; preds = %1773, %1791
  call void @free(ptr noundef nonnull %1763) #93, !noalias !39271
  br label %1793

1793:                                             ; preds = %__rustc::__rust_dealloc (.exit94), %1758
  %1794 = getelementptr inbounds nuw i8, ptr %58, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !39280)
  %1795 = load ptr, ptr %1794, align 8, !alias.scope !39283, !noundef !1740
  %1796 = icmp eq ptr %1795, null
  br i1 %1796, label %1456, label %1797

1797:                                             ; preds = %1793
  %1798 = atomicrmw sub ptr %1795, i64 1 release, align 8, !noalias !39284
  %1799 = icmp eq i64 %1798, 1
  br i1 %1799, label %1800, label %1456

1800:                                             ; preds = %1797
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1794) #92
          to label %1456 unwind label %1693
}
