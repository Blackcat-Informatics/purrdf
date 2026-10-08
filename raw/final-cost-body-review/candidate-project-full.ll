define void @purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %3, i64 noundef range(i64 0, 576460752303423488) %4, ptr noalias nofree noundef align 16 dereferenceable(1248) %5) unnamed_addr #8 personality ptr @rust_eh_personality !guid !36486 {
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
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36487)
  %28 = getelementptr inbounds nuw i8, ptr %5, i64 1120
  %29 = getelementptr inbounds nuw i8, ptr %5, i64 1136
  %30 = load i64, ptr %29, align 16, !alias.scope !36487, !noalias !36490, !noundef !1740
  %31 = icmp ult i64 %30, 192153584101141163
  tail call void @llvm.assume(i1 %31)
  %32 = icmp eq i64 %30, 0
  br i1 %32, label %33, label %34

33:                                               ; preds = %6
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %20, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %272 unwind label %270, !inline_history !36494

34:                                               ; preds = %6
  %35 = getelementptr inbounds nuw i8, ptr %5, i64 1128
  %36 = load ptr, ptr %35, align 8, !alias.scope !36487, !noalias !36490, !nonnull !1740, !noundef !1740
  %37 = mul nuw nsw i64 %30, 48
  %38 = getelementptr inbounds nuw i8, ptr %36, i64 %37
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !36495
  %39 = shl nuw nsw i64 %4, 4
  %40 = getelementptr inbounds nuw i8, ptr %3, i64 %39
  %41 = icmp eq i64 %4, 0
  br i1 %41, label %42, label %.preheader46

42:                                               ; preds = %34
  %43 = getelementptr inbounds nuw i8, ptr %36, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36502)
  %44 = getelementptr i8, ptr %36, i64 24
  %45 = load ptr, ptr %44, align 8, !noalias !36505
  %46 = getelementptr i8, ptr %36, i64 32
  %47 = load i64, ptr %46, align 8, !noalias !36505
  br label %.loopexit45

.preheader46:                                     ; preds = %34, %73
  %48 = phi ptr [ %49, %73 ], [ %36, %34 ]
  %49 = getelementptr inbounds nuw i8, ptr %48, i64 48
  %50 = getelementptr inbounds nuw i8, ptr %48, i64 24
  %51 = load ptr, ptr %50, align 8, !noalias !36508, !nonnull !1740, !noundef !1740
  %52 = getelementptr i8, ptr %48, i64 32
  %53 = load i64, ptr %52, align 8, !noalias !36508
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36502)
  %54 = getelementptr inbounds nuw i8, ptr %51, i64 16
  br label %55

55:                                               ; preds = %71, %.preheader46
  %56 = phi ptr [ %3, %.preheader46 ], [ %57, %71 ]
  %57 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %58 = load ptr, ptr %56, align 8, !alias.scope !36502, !noalias !36514, !nonnull !1740, !noundef !1740
  %59 = getelementptr i8, ptr %56, i64 8
  %60 = load i64, ptr %59, align 8, !alias.scope !36502, !noalias !36514, !noundef !1740
  %61 = icmp eq ptr %58, %51
  %62 = icmp eq i64 %60, %53
  %63 = xor i1 %62, true
  %64 = or i1 %61, %63
  br i1 %64, label %69, label %65

65:                                               ; preds = %55
  %66 = getelementptr inbounds nuw i8, ptr %58, i64 16
  %67 = tail call i32 @bcmp(ptr nonnull readonly %66, ptr nonnull readonly %54, i64 %53), !alias.scope !36517, !noalias !36521
  %68 = icmp eq i32 %67, 0
  br i1 %68, label %73, label %71

69:                                               ; preds = %55
  %70 = and i1 %61, %62
  br i1 %70, label %73, label %71

71:                                               ; preds = %69, %65
  %72 = icmp eq ptr %57, %40
  br i1 %72, label %.loopexit45, label %55

73:                                               ; preds = %69, %65
  %74 = icmp eq ptr %49, %38
  br i1 %74, label %.thread, label %.preheader46

.thread:                                          ; preds = %73
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !36495
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !36522
  store ptr inttoptr (i64 8 to ptr), ptr %12, align 8, !noalias !36522
  %75 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %76 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 0, ptr %76, align 8, !noalias !36522
  %77 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr inttoptr (i64 8 to ptr), ptr %77, align 8, !noalias !36522
  br label %.loopexit38

.loopexit45:                                      ; preds = %71, %42
  %78 = phi i64 [ %47, %42 ], [ %53, %71 ]
  %79 = phi ptr [ %45, %42 ], [ %51, %71 ]
  %80 = phi ptr [ %43, %42 ], [ %49, %71 ]
  %81 = atomicrmw add ptr %79, i64 1 monotonic, align 8, !noalias !36505
  %82 = icmp slt i64 %81, 0
  br i1 %82, label %83, label %89

83:                                               ; preds = %.loopexit45
  tail call void @llvm.trap()
  unreachable

84:                                               ; preds = %__rustc::__rust_alloc (.exit.thread)
  %85 = landingpad { ptr, i32 }
          cleanup
  %86 = atomicrmw sub ptr %79, i64 1 release, align 8, !noalias !36523
  %87 = icmp eq i64 %86, 1
  br i1 %87, label %88, label %687

88:                                               ; preds = %84
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %8) #91
          to label %687 unwind label %219, !noalias !36495

89:                                               ; preds = %.loopexit45
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !36495
  store ptr %79, ptr %8, align 8, !noalias !36495
  %90 = getelementptr inbounds nuw i8, ptr %8, i64 8
  store i64 %78, ptr %90, align 8, !noalias !36495
  %91 = tail call noundef dereferenceable_or_null(64) ptr @malloc(i64 noundef range(i64 1, 0) 64) #92, !noalias !36530
  %92 = icmp eq ptr %91, null
  br i1 %92, label %__rustc::__rust_alloc (.exit.thread), label %93

93:                                               ; preds = %89
  %94 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %95 = load i64, ptr %94, align 8, !noalias !36530, !noundef !1740
  %96 = tail call i64 @llvm.uadd.sat.i64(i64 %95, i64 1)
  store i64 %96, ptr %94, align 8, !noalias !36530
  %97 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %98 = load i64, ptr %97, align 8, !noalias !36530, !noundef !1740
  %99 = tail call i64 @llvm.uadd.sat.i64(i64 %98, i64 64)
  store i64 %99, ptr %97, align 8, !noalias !36530
  %100 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %101 = load i64, ptr %100, align 8, !noalias !36530, !noundef !1740
  %102 = tail call i64 @llvm.sadd.sat.i64(i64 %101, i64 64)
  store i64 %102, ptr %100, align 8, !noalias !36530
  %103 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %104 = load i64, ptr %103, align 8, !noalias !36530, !noundef !1740
  %105 = icmp sgt i64 %102, %104
  br i1 %105, label %106, label %.preheader241

106:                                              ; preds = %93
  store i64 %102, ptr %103, align 8, !noalias !36530
  br label %.preheader241

.preheader241:                                    ; preds = %106, %93
  br label %107

107:                                              ; preds = %.preheader241, %110
  %108 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36530
  %109 = icmp slt i64 %108, 0
  br i1 %109, label %110, label %__rustc::__rust_alloc (.exit)

110:                                              ; preds = %107
  %111 = add nsw i64 %108, 1
  %112 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %108, i64 %111 acq_rel acquire, align 8, !noalias !36530
  %113 = extractvalue { i64, i1 } %112, 1
  br i1 %113, label %114, label %107

114:                                              ; preds = %110
  %115 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36530
  %116 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 64 monotonic, align 8, !noalias !36530
  %117 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 64 monotonic, align 8, !noalias !36530
  %118 = tail call i64 @llvm.sadd.sat.i64(i64 %117, i64 64)
  %119 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36530
  br label %120

120:                                              ; preds = %123, %114
  %121 = phi i64 [ %119, %114 ], [ %126, %123 ]
  %122 = icmp sgt i64 %118, %121
  br i1 %122, label %123, label %127

123:                                              ; preds = %120
  %124 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %121, i64 %118 monotonic monotonic, align 8, !noalias !36530
  %125 = extractvalue { i64, i1 } %124, 1
  %126 = extractvalue { i64, i1 } %124, 0
  br i1 %125, label %127, label %120

127:                                              ; preds = %123, %120
  %128 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36530
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %89
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 64) #93
          to label %129 unwind label %84, !noalias !36495

129:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %107, %127
  store ptr %79, ptr %91, align 8, !noalias !36495
  %130 = getelementptr inbounds nuw i8, ptr %91, i64 8
  store i64 %78, ptr %130, align 8, !noalias !36495
  store i64 4, ptr %9, align 8, !noalias !36495
  %131 = getelementptr inbounds nuw i8, ptr %9, i64 8
  store ptr %91, ptr %131, align 8, !noalias !36495
  %132 = getelementptr inbounds nuw i8, ptr %9, i64 16
  store i64 1, ptr %132, align 8, !noalias !36495
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !36495
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36533)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36536)
  %133 = icmp eq ptr %80, %38
  br i1 %133, label %.loopexit40, label %134

134:                                              ; preds = %__rustc::__rust_alloc (.exit)
  %135 = getelementptr inbounds nuw i8, ptr %7, i64 8
  br i1 %41, label %.preheader, label %.preheader42

.preheader:                                       ; preds = %134, %153
  %136 = phi ptr [ %154, %153 ], [ %91, %134 ]
  %137 = phi i64 [ %157, %153 ], [ 1, %134 ]
  %138 = phi ptr [ %139, %153 ], [ %80, %134 ]
  %139 = getelementptr inbounds nuw i8, ptr %138, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36539)
  %140 = getelementptr i8, ptr %138, i64 24
  %141 = load ptr, ptr %140, align 8, !noalias !36542, !nonnull !1740, !noundef !1740
  %142 = getelementptr i8, ptr %138, i64 32
  %143 = load i64, ptr %142, align 8, !noalias !36542
  %144 = atomicrmw add ptr %141, i64 1 monotonic, align 8, !noalias !36542
  %145 = icmp slt i64 %144, 0
  br i1 %145, label %.loopexit39, label %146

146:                                              ; preds = %.preheader
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !36547
  store ptr %141, ptr %7, align 8, !noalias !36547
  store i64 %143, ptr %135, align 8, !noalias !36547
  %147 = icmp samesign ult i64 %137, 576460752303423488
  tail call void @llvm.assume(i1 %147)
  %148 = load i64, ptr %9, align 8, !range !1835, !alias.scope !36548, !noalias !36549, !noundef !1740
  %149 = icmp eq i64 %137, %148
  br i1 %149, label %150, label %153

150:                                              ; preds = %146
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %137, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %151 unwind label %159, !noalias !36549

151:                                              ; preds = %150
  %152 = load ptr, ptr %131, align 8, !alias.scope !36548, !noalias !36549
  br label %153

153:                                              ; preds = %151, %146
  %154 = phi ptr [ %152, %151 ], [ %136, %146 ]
  %155 = getelementptr inbounds nuw [16 x i8], ptr %154, i64 %137
  store ptr %141, ptr %155, align 8, !noalias !36547
  %156 = getelementptr inbounds nuw i8, ptr %155, i64 8
  store i64 %143, ptr %156, align 8, !noalias !36547
  %157 = add nuw nsw i64 %137, 1
  store i64 %157, ptr %132, align 8, !alias.scope !36548, !noalias !36549
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !36547
  %158 = icmp eq ptr %139, %38
  br i1 %158, label %.loopexit40, label %.preheader

159:                                              ; preds = %150
  %160 = landingpad { ptr, i32 }
          cleanup
  br label %207

.preheader42:                                     ; preds = %134, %199
  %161 = phi ptr [ %200, %199 ], [ %91, %134 ]
  %162 = phi i64 [ %203, %199 ], [ 1, %134 ]
  %163 = phi ptr [ %166, %199 ], [ %80, %134 ]
  br label %164

164:                                              ; preds = %190, %.preheader42
  %165 = phi ptr [ %166, %190 ], [ %163, %.preheader42 ]
  %166 = getelementptr inbounds nuw i8, ptr %165, i64 48
  %167 = getelementptr i8, ptr %165, i64 24
  %168 = load ptr, ptr %167, align 8, !noalias !36550, !nonnull !1740, !noundef !1740
  %169 = getelementptr i8, ptr %165, i64 32
  %170 = load i64, ptr %169, align 8, !noalias !36550
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36539)
  %171 = getelementptr inbounds nuw i8, ptr %168, i64 16
  br label %172

172:                                              ; preds = %188, %164
  %173 = phi ptr [ %3, %164 ], [ %174, %188 ]
  %174 = getelementptr inbounds nuw i8, ptr %173, i64 16
  %175 = load ptr, ptr %173, align 8, !alias.scope !36539, !noalias !36556, !nonnull !1740, !noundef !1740
  %176 = getelementptr i8, ptr %173, i64 8
  %177 = load i64, ptr %176, align 8, !alias.scope !36539, !noalias !36556, !noundef !1740
  %178 = icmp eq ptr %175, %168
  %179 = icmp eq i64 %177, %170
  %180 = xor i1 %179, true
  %181 = or i1 %178, %180
  br i1 %181, label %186, label %182

182:                                              ; preds = %172
  %183 = getelementptr inbounds nuw i8, ptr %175, i64 16
  %184 = tail call i32 @bcmp(ptr nonnull readonly %183, ptr nonnull readonly %171, i64 %170), !alias.scope !36559, !noalias !36563
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
  br i1 %191, label %.loopexit40, label %164

192:                                              ; preds = %188
  %193 = atomicrmw add ptr %168, i64 1 monotonic, align 8, !noalias !36542
  %194 = icmp slt i64 %193, 0
  br i1 %194, label %.loopexit39, label %195

.loopexit39:                                      ; preds = %192, %.preheader
  tail call void @llvm.trap()
  unreachable

195:                                              ; preds = %192
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !36547
  store ptr %168, ptr %7, align 8, !noalias !36547
  store i64 %170, ptr %135, align 8, !noalias !36547
  %196 = icmp samesign ult i64 %162, 576460752303423488
  tail call void @llvm.assume(i1 %196)
  %197 = load i64, ptr %9, align 8, !range !1835, !alias.scope !36548, !noalias !36549, !noundef !1740
  %198 = icmp eq i64 %162, %197
  br i1 %198, label %213, label %199

199:                                              ; preds = %214, %195
  %200 = phi ptr [ %215, %214 ], [ %161, %195 ]
  %201 = getelementptr inbounds nuw [16 x i8], ptr %200, i64 %162
  store ptr %168, ptr %201, align 8, !noalias !36547
  %202 = getelementptr inbounds nuw i8, ptr %201, i64 8
  store i64 %170, ptr %202, align 8, !noalias !36547
  %203 = add nuw nsw i64 %162, 1
  store i64 %203, ptr %132, align 8, !alias.scope !36548, !noalias !36549
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !36547
  %204 = icmp eq ptr %166, %38
  br i1 %204, label %.loopexit40, label %.preheader42

205:                                              ; preds = %213
  %206 = landingpad { ptr, i32 }
          cleanup
  br label %207

207:                                              ; preds = %205, %159
  %208 = phi ptr [ %168, %205 ], [ %141, %159 ]
  %209 = phi { ptr, i32 } [ %206, %205 ], [ %160, %159 ]
  %210 = atomicrmw sub ptr %208, i64 1 release, align 8, !noalias !36564
  %211 = icmp eq i64 %210, 1
  br i1 %211, label %212, label %218

212:                                              ; preds = %207
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7) #91
          to label %218 unwind label %216, !noalias !36547

213:                                              ; preds = %195
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %162, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %214 unwind label %205, !noalias !36549

214:                                              ; preds = %213
  %215 = load ptr, ptr %131, align 8, !alias.scope !36548, !noalias !36549
  br label %199

216:                                              ; preds = %212
  %217 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36547
  unreachable

218:                                              ; preds = %212, %207
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
  invoke void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9) #89
          to label %687 unwind label %219, !noalias !36495

219:                                              ; preds = %218, %88
  %220 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36495
  unreachable

.loopexit40:                                      ; preds = %199, %190, %153, %__rustc::__rust_alloc (.exit)
  %221 = phi i64 [ %162, %190 ], [ %157, %153 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %203, %199 ]
  %222 = load i64, ptr %9, align 8, !noalias !36571
  %223 = load ptr, ptr %131, align 8, !noalias !36571
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !36495
  %224 = icmp ult i64 %221, 576460752303423488
  tail call void @llvm.assume(i1 %224)
  %225 = shl nuw nsw i64 %221, 4
  %226 = getelementptr inbounds nuw i8, ptr %223, i64 %225
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !36522
  store ptr %223, ptr %12, align 8, !noalias !36522
  %227 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %228 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %222, ptr %228, align 8, !noalias !36522
  %229 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr %226, ptr %229, align 8, !noalias !36522
  %230 = getelementptr inbounds nuw i8, ptr %11, i64 24
  %231 = getelementptr inbounds nuw i8, ptr %11, i64 32
  %232 = getelementptr inbounds nuw i8, ptr %11, i64 40
  %233 = load i64, ptr %29, align 16, !alias.scope !36572, !noalias !36575
  br label %235

234:                                              ; preds = %245
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12) #89
          to label %687 unwind label %262, !noalias !36577, !inline_history !36578

235:                                              ; preds = %264, %.loopexit40
  %236 = phi i64 [ %233, %.loopexit40 ], [ %267, %264 ]
  %237 = phi ptr [ %223, %.loopexit40 ], [ %238, %264 ]
  %238 = getelementptr inbounds nuw i8, ptr %237, i64 16
  %239 = load ptr, ptr %237, align 8, !noalias !36579, !nonnull !1740, !noundef !1740
  %240 = getelementptr inbounds nuw i8, ptr %237, i64 8
  %241 = load i64, ptr %240, align 8, !noalias !36579, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !36522
  store ptr %239, ptr %230, align 8, !noalias !36522
  store i64 %241, ptr %231, align 8, !noalias !36522
  store i64 2, ptr %11, align 8, !noalias !36522
  store i8 0, ptr %232, align 8, !noalias !36522
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36572)
  %242 = load i64, ptr %28, align 16, !range !1835, !alias.scope !36572, !noalias !36575, !noundef !1740
  %243 = icmp eq i64 %236, %242
  br i1 %243, label %244, label %264

244:                                              ; preds = %235
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %28)
          to label %264 unwind label %245, !noalias !36575

245:                                              ; preds = %244
  %246 = landingpad { ptr, i32 }
          cleanup
  store ptr %238, ptr %227, align 8
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(48) %11) #89
          to label %234 unwind label %247, !noalias !36582

247:                                              ; preds = %245
  %248 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36582
  unreachable

.loopexit38:                                      ; preds = %264, %.thread
  %249 = phi ptr [ %75, %.thread ], [ %227, %264 ]
  %250 = phi ptr [ inttoptr (i64 8 to ptr), %.thread ], [ %226, %264 ]
  store ptr %250, ptr %249, align 1
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12)
          to label %251 unwind label %270, !inline_history !36578

251:                                              ; preds = %.loopexit38
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !36522
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !36522
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %10, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %252 unwind label %270, !inline_history !36494

252:                                              ; preds = %251
  %253 = load i64, ptr %29, align 16, !alias.scope !36583, !noalias !36586, !noundef !1740
  %254 = icmp ugt i64 %30, %253
  br i1 %254, label %261, label %255

255:                                              ; preds = %252
  %256 = sub nuw i64 %253, %30
  %257 = load ptr, ptr %35, align 8, !alias.scope !36583, !noalias !36586, !nonnull !1740, !noundef !1740
  %258 = getelementptr inbounds nuw [48 x i8], ptr %257, i64 %30
  store i64 %30, ptr %29, align 16, !alias.scope !36583, !noalias !36586
; invoke core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
  invoke fastcc void @core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>(ptr noalias nofree noundef nonnull align 8 %258, i64 noundef %256)
          to label %261 unwind label %259

259:                                              ; preds = %255
  %260 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef align 16 dereferenceable(112) %10) #89
          to label %687 unwind label %262, !noalias !36586, !inline_history !36578

261:                                              ; preds = %255, %252
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(112) %20, ptr noundef nonnull align 16 dereferenceable(112) %10, i64 112, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !36522
  br label %272

262:                                              ; preds = %259, %234
  %263 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90, !noalias !36586, !inline_history !36578
  unreachable

264:                                              ; preds = %244, %235
  %265 = load ptr, ptr %35, align 8, !alias.scope !36572, !noalias !36575, !nonnull !1740, !noundef !1740
  %266 = getelementptr inbounds nuw [48 x i8], ptr %265, i64 %236
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %266, ptr noundef nonnull align 8 dereferenceable(48) %11, i64 48, i1 false), !noalias !36582
  %267 = add i64 %236, 1
  store i64 %267, ptr %29, align 16, !alias.scope !36572, !noalias !36575
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !36522
  %268 = icmp eq ptr %238, %226
  br i1 %268, label %.loopexit38, label %235

269:                                              ; preds = %369
  br i1 %370, label %687, label %685

270:                                              ; preds = %365, %358, %251, %.loopexit38, %33
  %271 = landingpad { ptr, i32 }
          cleanup
  br label %687

272:                                              ; preds = %261, %33
  %273 = load i64, ptr %20, align 16, !range !1739, !noundef !1740
  %274 = trunc nuw i64 %273 to i1
  br i1 %274, label %275, label %358

275:                                              ; preds = %272
  %276 = getelementptr inbounds nuw i8, ptr %20, i64 16
  %277 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %277, ptr noundef nonnull align 16 dereferenceable(96) %276, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36587)
  %278 = getelementptr inbounds nuw i8, ptr %27, i64 72
  %279 = load i64, ptr %278, align 8, !range !1778, !alias.scope !36590, !noundef !1740
  %280 = icmp ugt i64 %279, 5
  br i1 %280, label %281, label %315

281:                                              ; preds = %275
  %282 = getelementptr inbounds nuw i8, ptr %27, i64 80
  %283 = load ptr, ptr %282, align 8, !alias.scope !36587, !nonnull !1740, !noundef !1740
  %284 = mul i64 %279, 3
  %285 = add i64 %284, -3
  %286 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %287 = load i64, ptr %286, align 8, !noalias !36593, !noundef !1740
  %288 = tail call i64 @llvm.umin.i64(i64 %285, i64 9223372036854775807)
  %289 = tail call i64 @llvm.ssub.sat.i64(i64 %287, i64 %288)
  store i64 %289, ptr %286, align 8, !noalias !36593
  %290 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %291 = load i64, ptr %290, align 8, !noalias !36593, !noundef !1740
  %292 = icmp slt i64 %289, %291
  br i1 %292, label %293, label %.preheader214

293:                                              ; preds = %281
  store i64 %289, ptr %290, align 8, !noalias !36593
  br label %.preheader214

.preheader214:                                    ; preds = %293, %281
  br label %294

294:                                              ; preds = %.preheader214, %297
  %295 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36593
  %296 = icmp slt i64 %295, 0
  br i1 %296, label %297, label %__rustc::__rust_dealloc (.exit)

297:                                              ; preds = %294
  %298 = add nsw i64 %295, 1
  %299 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %295, i64 %298 acq_rel acquire, align 8, !noalias !36593
  %300 = extractvalue { i64, i1 } %299, 1
  br i1 %300, label %301, label %294

301:                                              ; preds = %297
  %302 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %288 monotonic, align 8, !noalias !36593
  %303 = tail call i64 @llvm.ssub.sat.i64(i64 %302, i64 %288)
  %304 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36593
  br label %305

305:                                              ; preds = %308, %301
  %306 = phi i64 [ %304, %301 ], [ %311, %308 ]
  %307 = icmp slt i64 %303, %306
  br i1 %307, label %308, label %312

308:                                              ; preds = %305
  %309 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %306, i64 %303 monotonic monotonic, align 8, !noalias !36593
  %310 = extractvalue { i64, i1 } %309, 1
  %311 = extractvalue { i64, i1 } %309, 0
  br i1 %310, label %312, label %305

312:                                              ; preds = %308, %305
  %313 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36593
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %294, %312
  %314 = icmp ne i64 %285, 0
  tail call void @llvm.assume(i1 %314), !noalias !36593
  tail call void @free(ptr noundef nonnull %283) #92, !noalias !36593
  br label %315

315:                                              ; preds = %__rustc::__rust_dealloc (.exit), %275
  %316 = load i64, ptr %27, align 8, !range !2059, !alias.scope !36587, !noundef !1740
  %317 = icmp sgt i64 %316, 0
  br i1 %317, label %318, label %350

318:                                              ; preds = %315
  %319 = getelementptr inbounds nuw i8, ptr %27, i64 8
  %320 = load ptr, ptr %319, align 8, !alias.scope !36587, !nonnull !1740, !noundef !1740
  %321 = mul nuw i64 %316, 3
  %322 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %323 = load i64, ptr %322, align 8, !noalias !36587, !noundef !1740
  %324 = tail call i64 @llvm.umin.i64(i64 %321, i64 9223372036854775807)
  %325 = tail call i64 @llvm.ssub.sat.i64(i64 %323, i64 %324)
  store i64 %325, ptr %322, align 8, !noalias !36587
  %326 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %327 = load i64, ptr %326, align 8, !noalias !36587, !noundef !1740
  %328 = icmp slt i64 %325, %327
  br i1 %328, label %329, label %.preheader213

329:                                              ; preds = %318
  store i64 %325, ptr %326, align 8, !noalias !36587
  br label %.preheader213

.preheader213:                                    ; preds = %329, %318
  br label %330

330:                                              ; preds = %.preheader213, %333
  %331 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36587
  %332 = icmp slt i64 %331, 0
  br i1 %332, label %333, label %__rustc::__rust_dealloc (.exit31)

333:                                              ; preds = %330
  %334 = add nsw i64 %331, 1
  %335 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %331, i64 %334 acq_rel acquire, align 8, !noalias !36587
  %336 = extractvalue { i64, i1 } %335, 1
  br i1 %336, label %337, label %330

337:                                              ; preds = %333
  %338 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %324 monotonic, align 8, !noalias !36587
  %339 = tail call i64 @llvm.ssub.sat.i64(i64 %338, i64 %324)
  %340 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36587
  br label %341

341:                                              ; preds = %344, %337
  %342 = phi i64 [ %340, %337 ], [ %347, %344 ]
  %343 = icmp slt i64 %339, %342
  br i1 %343, label %344, label %348

344:                                              ; preds = %341
  %345 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %342, i64 %339 monotonic monotonic, align 8, !noalias !36587
  %346 = extractvalue { i64, i1 } %345, 1
  %347 = extractvalue { i64, i1 } %345, 0
  br i1 %346, label %348, label %341

348:                                              ; preds = %344, %341
  %349 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36587
  br label %__rustc::__rust_dealloc (.exit31)

__rustc::__rust_dealloc (.exit31): ; preds = %330, %348
  tail call void @free(ptr noundef nonnull %320) #92, !noalias !36587
  br label %350

350:                                              ; preds = %__rustc::__rust_dealloc (.exit31), %315
  %351 = getelementptr inbounds nuw i8, ptr %27, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36596)
  %352 = load ptr, ptr %351, align 8, !alias.scope !36599, !noundef !1740
  %353 = icmp eq ptr %352, null
  br i1 %353, label %625, label %354

354:                                              ; preds = %350
  %355 = atomicrmw sub ptr %352, i64 1 release, align 8, !noalias !36600
  %356 = icmp eq i64 %355, 1
  br i1 %356, label %357, label %625

357:                                              ; preds = %354
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %351) #91
  br label %625

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
          to label %628 unwind label %270

367:                                              ; preds = %363
  %368 = landingpad { ptr, i32 }
          cleanup
  br label %369

369:                                              ; preds = %623, %620, %436, %367
  %370 = phi i1 [ true, %367 ], [ false, %436 ], [ false, %623 ], [ false, %620 ]
  %371 = phi { ptr, i32 } [ %368, %367 ], [ %488, %436 ], [ %619, %623 ], [ %619, %620 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23) #89
          to label %269 unwind label %626

372:                                              ; preds = %363
  call void @llvm.lifetime.start.p0(ptr nonnull %21)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %21, ptr noundef nonnull align 8 dereferenceable(104) %27, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36605)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36608)
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  store ptr %364, ptr %19, align 8, !noalias !36610
  %373 = getelementptr inbounds nuw i8, ptr %364, i64 24
  %374 = load ptr, ptr %373, align 8, !noalias !36610, !nonnull !1740, !noundef !1740
  %375 = getelementptr inbounds nuw i8, ptr %364, i64 32
  %376 = load i64, ptr %375, align 8, !noalias !36610, !noundef !1740
  %377 = shl nuw i64 %376, 4
  %378 = icmp eq i64 %376, 0
  br i1 %378, label %.loopexit37, label %379

379:                                              ; preds = %372
  %380 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %377) #92, !noalias !36612
  %381 = icmp eq ptr %380, null
  br i1 %381, label %__rustc::__rust_alloc (.exit32.thread), label %382

382:                                              ; preds = %379
  %383 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %384 = load i64, ptr %383, align 8, !noalias !36612, !noundef !1740
  %385 = tail call i64 @llvm.uadd.sat.i64(i64 %384, i64 1)
  store i64 %385, ptr %383, align 8, !noalias !36612
  %386 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %387 = load i64, ptr %386, align 8, !noalias !36612, !noundef !1740
  %388 = tail call i64 @llvm.uadd.sat.i64(i64 %387, i64 %377)
  store i64 %388, ptr %386, align 8, !noalias !36612
  %389 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %390 = load i64, ptr %389, align 8, !noalias !36612, !noundef !1740
  %391 = tail call i64 @llvm.umin.i64(i64 %377, i64 9223372036854775807)
  %392 = tail call i64 @llvm.sadd.sat.i64(i64 %390, i64 %391)
  store i64 %392, ptr %389, align 8, !noalias !36612
  %393 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %394 = load i64, ptr %393, align 8, !noalias !36612, !noundef !1740
  %395 = icmp sgt i64 %392, %394
  br i1 %395, label %396, label %.preheader219

396:                                              ; preds = %382
  store i64 %392, ptr %393, align 8, !noalias !36612
  br label %.preheader219

.preheader219:                                    ; preds = %396, %382
  br label %397

397:                                              ; preds = %.preheader219, %400
  %398 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36612
  %399 = icmp slt i64 %398, 0
  br i1 %399, label %400, label %__rustc::__rust_alloc (.exit32)

400:                                              ; preds = %397
  %401 = add nsw i64 %398, 1
  %402 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %398, i64 %401 acq_rel acquire, align 8, !noalias !36612
  %403 = extractvalue { i64, i1 } %402, 1
  br i1 %403, label %404, label %397

404:                                              ; preds = %400
  %405 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36612
  %406 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %377 monotonic, align 8, !noalias !36612
  %407 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %391 monotonic, align 8, !noalias !36612
  %408 = tail call i64 @llvm.sadd.sat.i64(i64 %407, i64 %391)
  %409 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36612
  br label %410

410:                                              ; preds = %413, %404
  %411 = phi i64 [ %409, %404 ], [ %416, %413 ]
  %412 = icmp sgt i64 %408, %411
  br i1 %412, label %413, label %417

413:                                              ; preds = %410
  %414 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %411, i64 %408 monotonic monotonic, align 8, !noalias !36612
  %415 = extractvalue { i64, i1 } %414, 1
  %416 = extractvalue { i64, i1 } %414, 0
  br i1 %415, label %417, label %410

417:                                              ; preds = %413, %410
  %418 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36612
  br label %__rustc::__rust_alloc (.exit32)

__rustc::__rust_alloc (.exit32.thread): ; preds = %379
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %377) #93
          to label %419 unwind label %437

419:                                              ; preds = %__rustc::__rust_alloc (.exit32.thread)
  unreachable

__rustc::__rust_alloc (.exit32): ; preds = %397, %417
  %420 = getelementptr inbounds nuw i8, ptr %23, i64 24
  %421 = load ptr, ptr %420, align 8, !noalias !36621, !nonnull !1740, !noundef !1740
  %422 = getelementptr inbounds nuw i8, ptr %421, i64 16
  br label %423

423:                                              ; preds = %427, %__rustc::__rust_alloc (.exit32)
  %424 = phi i64 [ 0, %__rustc::__rust_alloc (.exit32) ], [ %432, %427 ]
  %425 = getelementptr inbounds nuw [16 x i8], ptr %374, i64 %424
; invoke <purrdf_sparql_eval::solution::VarSchema>::index_of
  %426 = invoke fastcc { i64, i64 } @<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.12908414067662811932)(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %422, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %425) #87
          to label %427 unwind label %434, !noalias !36641

427:                                              ; preds = %423
  %428 = extractvalue { i64, i64 } %426, 0
  %429 = extractvalue { i64, i64 } %426, 1
  %430 = getelementptr inbounds nuw [16 x i8], ptr %380, i64 %424
  store i64 %428, ptr %430, align 8, !noalias !36642
  %431 = getelementptr inbounds nuw i8, ptr %430, i64 8
  store i64 %429, ptr %431, align 8, !noalias !36642
  %432 = add nuw i64 %424, 1
  %433 = icmp eq i64 %432, %376
  br i1 %433, label %.loopexit37, label %423

434:                                              ; preds = %423
  %435 = landingpad { ptr, i32 }
          cleanup
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %380, i64 noundef %377, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !36647
  br label %618

436:                                              ; preds = %490, %487
  br i1 %489, label %369, label %618

437:                                              ; preds = %__rustc::__rust_alloc (.exit32.thread)
  %438 = landingpad { ptr, i32 }
          cleanup
  br label %618

.loopexit37:                                      ; preds = %427, %372
  %439 = phi ptr [ inttoptr (i64 8 to ptr), %372 ], [ %380, %427 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !36610
  %440 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %441 = load i64, ptr %440, align 8, !alias.scope !36608, !noalias !36648, !noundef !1740
  %442 = icmp ult i64 %441, 230584300921369396
  tail call void @llvm.assume(i1 %442)
  %443 = mul nuw nsw i64 %441, 40
  %444 = icmp eq i64 %441, 0
  br i1 %444, label %445, label %448

445:                                              ; preds = %.loopexit37
  store i64 0, ptr %18, align 8, !noalias !36610
  %446 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %446, align 8, !noalias !36610
  %447 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 0, ptr %447, align 8, !noalias !36610
  br label %.loopexit

448:                                              ; preds = %.loopexit37
  %449 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %443) #92, !noalias !36649
  %450 = icmp eq ptr %449, null
  br i1 %450, label %__rustc::__rust_alloc (.exit33.thread), label %451

451:                                              ; preds = %448
  %452 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %453 = load i64, ptr %452, align 8, !noalias !36649, !noundef !1740
  %454 = tail call i64 @llvm.uadd.sat.i64(i64 %453, i64 1)
  store i64 %454, ptr %452, align 8, !noalias !36649
  %455 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %456 = load i64, ptr %455, align 8, !noalias !36649, !noundef !1740
  %457 = tail call i64 @llvm.uadd.sat.i64(i64 %456, i64 %443)
  store i64 %457, ptr %455, align 8, !noalias !36649
  %458 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %459 = load i64, ptr %458, align 8, !noalias !36649, !noundef !1740
  %460 = tail call i64 @llvm.sadd.sat.i64(i64 %459, i64 %443)
  store i64 %460, ptr %458, align 8, !noalias !36649
  %461 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %462 = load i64, ptr %461, align 8, !noalias !36649, !noundef !1740
  %463 = icmp sgt i64 %460, %462
  br i1 %463, label %464, label %.preheader218

464:                                              ; preds = %451
  store i64 %460, ptr %461, align 8, !noalias !36649
  br label %.preheader218

.preheader218:                                    ; preds = %464, %451
  br label %465

465:                                              ; preds = %.preheader218, %468
  %466 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36649
  %467 = icmp slt i64 %466, 0
  br i1 %467, label %468, label %__rustc::__rust_alloc (.exit33)

468:                                              ; preds = %465
  %469 = add nsw i64 %466, 1
  %470 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %466, i64 %469 acq_rel acquire, align 8, !noalias !36649
  %471 = extractvalue { i64, i1 } %470, 1
  br i1 %471, label %472, label %465

472:                                              ; preds = %468
  %473 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !36649
  %474 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %443 monotonic, align 8, !noalias !36649
  %475 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %443 monotonic, align 8, !noalias !36649
  %476 = tail call i64 @llvm.sadd.sat.i64(i64 %475, i64 %443)
  %477 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !36649
  br label %478

478:                                              ; preds = %481, %472
  %479 = phi i64 [ %477, %472 ], [ %484, %481 ]
  %480 = icmp sgt i64 %476, %479
  br i1 %480, label %481, label %485

481:                                              ; preds = %478
  %482 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %479, i64 %476 monotonic monotonic, align 8, !noalias !36649
  %483 = extractvalue { i64, i1 } %482, 1
  %484 = extractvalue { i64, i1 } %482, 0
  br i1 %483, label %485, label %478

485:                                              ; preds = %481, %478
  %486 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36649
  br label %__rustc::__rust_alloc (.exit33)

487:                                              ; preds = %613, %508, %491
  %488 = phi { ptr, i32 } [ %492, %491 ], [ %614, %613 ], [ %509, %508 ]
  %489 = phi i1 [ false, %491 ], [ false, %613 ], [ true, %508 ]
  br i1 %378, label %436, label %490

490:                                              ; preds = %487
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %439, i64 noundef %377, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %436

491:                                              ; preds = %__rustc::__rust_alloc (.exit33.thread)
  %492 = landingpad { ptr, i32 }
          cleanup
  br label %487

__rustc::__rust_alloc (.exit33.thread): ; preds = %448
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %443) #93
          to label %617 unwind label %491, !noalias !36648

__rustc::__rust_alloc (.exit33): ; preds = %465, %485
  store i64 %441, ptr %18, align 8, !noalias !36610
  %493 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr %449, ptr %493, align 8, !noalias !36610
  %494 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 0, ptr %494, align 8, !noalias !36610
  %495 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %496 = load ptr, ptr %495, align 8, !alias.scope !36608, !noalias !36648, !nonnull !1740, !noundef !1740
  %497 = getelementptr inbounds nuw i8, ptr %496, i64 %443
  %498 = getelementptr inbounds nuw [16 x i8], ptr %439, i64 %376
  %499 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %500 = getelementptr inbounds nuw i8, ptr %16, i64 16
  br label %501

501:                                              ; preds = %525, %__rustc::__rust_alloc (.exit33)
  %502 = phi ptr [ %449, %__rustc::__rust_alloc (.exit33) ], [ %526, %525 ]
  %503 = phi i64 [ 0, %__rustc::__rust_alloc (.exit33) ], [ %528, %525 ]
  %504 = phi ptr [ %496, %__rustc::__rust_alloc (.exit33) ], [ %505, %525 ]
  %505 = getelementptr inbounds nuw i8, ptr %504, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !36610
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !36610
  store ptr %439, ptr %16, align 8, !noalias !36610
  store ptr %498, ptr %499, align 8, !noalias !36610
  store ptr %504, ptr %500, align 8, !noalias !36610
; invoke <purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::from_iter::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>
  invoke fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]> as core::iter::traits::collect::FromIterator<core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>>>::from_iter::<core::iter::adapters::map::Map<core::slice::iter::Iter<core::option::Option<usize>>, purrdf_sparql_eval::modifier::eval_project_sequence<purrdf_core::ir::term::TermId>::{closure#1}>>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(40) %17, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %16)
          to label %510 unwind label %506, !noalias !36648

506:                                              ; preds = %501
  %507 = landingpad { ptr, i32 }
          cleanup
  br label %613

508:                                              ; preds = %581
  %509 = landingpad { ptr, i32 }
          cleanup
  br label %487

510:                                              ; preds = %501
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !36610
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36652)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36655)
  %511 = load i64, ptr %18, align 8, !range !1835, !alias.scope !36652, !noalias !36657, !noundef !1740
  %512 = icmp eq i64 %503, %511
  br i1 %512, label %513, label %525

513:                                              ; preds = %510
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %18)
          to label %514 unwind label %516, !noalias !36657

514:                                              ; preds = %513
  %515 = load ptr, ptr %493, align 8, !alias.scope !36652, !noalias !36657
  br label %525

516:                                              ; preds = %513
  %517 = landingpad { ptr, i32 }
          cleanup
  %518 = load i64, ptr %17, align 8, !range !1778, !alias.scope !36658, !noalias !36661, !noundef !1740
  %519 = icmp ugt i64 %518, 5
  br i1 %519, label %520, label %613

520:                                              ; preds = %516
  %521 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %522 = load ptr, ptr %521, align 8, !alias.scope !36655, !noalias !36661, !nonnull !1740, !noundef !1740
  %523 = shl i64 %518, 3
  %524 = add i64 %523, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %522, i64 noundef %524, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !36662
  br label %613

525:                                              ; preds = %514, %510
  %526 = phi ptr [ %515, %514 ], [ %502, %510 ]
  %527 = getelementptr inbounds nuw [40 x i8], ptr %526, i64 %503
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %527, ptr noundef nonnull readonly align 8 dereferenceable(40) %17, i64 40, i1 false), !noalias !36661
  %528 = add nuw nsw i64 %503, 1
  store i64 %528, ptr %494, align 8, !alias.scope !36652, !noalias !36657
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !36610
  %529 = icmp eq ptr %505, %497
  br i1 %529, label %.loopexit, label %501

.loopexit:                                        ; preds = %525, %445
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !36610
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !36610
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %14, ptr noundef nonnull align 8 dereferenceable(104) %21, i64 104, i1 false), !noalias !36665
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !36610
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %13, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false), !noalias !36610
  %530 = getelementptr inbounds nuw i8, ptr %13, i64 24
  store ptr %364, ptr %530, align 8, !noalias !36610
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36666)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36669)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36671)
  %531 = load i64, ptr %14, align 8, !range !2059, !alias.scope !36669, !noalias !36673, !noundef !1740
  %532 = icmp eq i64 %531, -1
  br i1 %532, label %534, label %533

533:                                              ; preds = %.loopexit
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %15, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %13, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %21), !noalias !36605
  br label %536

534:                                              ; preds = %.loopexit
  %535 = getelementptr inbounds nuw i8, ptr %15, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %535, ptr noundef nonnull readonly align 8 dereferenceable(32) %13, i64 32, i1 false), !alias.scope !36674, !noalias !36675
  store i64 -1, ptr %15, align 8, !alias.scope !36666, !noalias !36676
  br label %536

536:                                              ; preds = %534, %533
  %537 = getelementptr inbounds nuw i8, ptr %14, i64 72
  %538 = load i64, ptr %537, align 8, !range !1778, !alias.scope !36677, !noalias !36673, !noundef !1740
  %539 = icmp ugt i64 %538, 5
  br i1 %539, label %540, label %574

540:                                              ; preds = %536
  %541 = getelementptr inbounds nuw i8, ptr %14, i64 80
  %542 = load ptr, ptr %541, align 8, !alias.scope !36669, !noalias !36673, !nonnull !1740, !noundef !1740
  %543 = mul i64 %538, 3
  %544 = add i64 %543, -3
  %545 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %546 = load i64, ptr %545, align 8, !noalias !36680, !noundef !1740
  %547 = tail call i64 @llvm.umin.i64(i64 %544, i64 9223372036854775807)
  %548 = tail call i64 @llvm.ssub.sat.i64(i64 %546, i64 %547)
  store i64 %548, ptr %545, align 8, !noalias !36680
  %549 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %550 = load i64, ptr %549, align 8, !noalias !36680, !noundef !1740
  %551 = icmp slt i64 %548, %550
  br i1 %551, label %552, label %.preheader217

552:                                              ; preds = %540
  store i64 %548, ptr %549, align 8, !noalias !36680
  br label %.preheader217

.preheader217:                                    ; preds = %552, %540
  br label %553

553:                                              ; preds = %.preheader217, %556
  %554 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36680
  %555 = icmp slt i64 %554, 0
  br i1 %555, label %556, label %__rustc::__rust_dealloc (.exit34)

556:                                              ; preds = %553
  %557 = add nsw i64 %554, 1
  %558 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %554, i64 %557 acq_rel acquire, align 8, !noalias !36680
  %559 = extractvalue { i64, i1 } %558, 1
  br i1 %559, label %560, label %553

560:                                              ; preds = %556
  %561 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %547 monotonic, align 8, !noalias !36680
  %562 = tail call i64 @llvm.ssub.sat.i64(i64 %561, i64 %547)
  %563 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36680
  br label %564

564:                                              ; preds = %567, %560
  %565 = phi i64 [ %563, %560 ], [ %570, %567 ]
  %566 = icmp slt i64 %562, %565
  br i1 %566, label %567, label %571

567:                                              ; preds = %564
  %568 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %565, i64 %562 monotonic monotonic, align 8, !noalias !36680
  %569 = extractvalue { i64, i1 } %568, 1
  %570 = extractvalue { i64, i1 } %568, 0
  br i1 %569, label %571, label %564

571:                                              ; preds = %567, %564
  %572 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36680
  br label %__rustc::__rust_dealloc (.exit34)

__rustc::__rust_dealloc (.exit34): ; preds = %553, %571
  %573 = icmp ne i64 %544, 0
  tail call void @llvm.assume(i1 %573), !noalias !36680
  tail call void @free(ptr noundef nonnull %542) #92, !noalias !36680
  br label %574

574:                                              ; preds = %__rustc::__rust_dealloc (.exit34), %536
  %575 = getelementptr inbounds nuw i8, ptr %14, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36683), !noalias !36648
  %576 = load ptr, ptr %575, align 8, !alias.scope !36686, !noalias !36673, !noundef !1740
  %577 = icmp eq ptr %576, null
  br i1 %577, label %582, label %578

578:                                              ; preds = %574
  %579 = atomicrmw sub ptr %576, i64 1 release, align 8, !noalias !36687
  %580 = icmp eq i64 %579, 1
  br i1 %580, label %581, label %582

581:                                              ; preds = %578
  fence acquire, !noalias !36648
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %575) #91
          to label %582 unwind label %508

582:                                              ; preds = %581, %578, %574
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !36610
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !36610
  %583 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %583, ptr noundef nonnull align 8 dereferenceable(96) %15, i64 96, i1 false), !noalias !36692
  store i64 0, ptr %0, align 16, !alias.scope !36605, !noalias !36692
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !36610
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !36610
  br i1 %378, label %624, label %584

584:                                              ; preds = %582
  %585 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %586 = load i64, ptr %585, align 8, !noundef !1740
  %587 = tail call i64 @llvm.umin.i64(i64 %377, i64 9223372036854775807)
  %588 = tail call i64 @llvm.ssub.sat.i64(i64 %586, i64 %587)
  store i64 %588, ptr %585, align 8
  %589 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %590 = load i64, ptr %589, align 8, !noundef !1740
  %591 = icmp slt i64 %588, %590
  br i1 %591, label %592, label %.preheader216

592:                                              ; preds = %584
  store i64 %588, ptr %589, align 8
  br label %.preheader216

.preheader216:                                    ; preds = %592, %584
  br label %593

593:                                              ; preds = %.preheader216, %596
  %594 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %595 = icmp slt i64 %594, 0
  br i1 %595, label %596, label %__rustc::__rust_dealloc (.exit35)

596:                                              ; preds = %593
  %597 = add nsw i64 %594, 1
  %598 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %594, i64 %597 acq_rel acquire, align 8
  %599 = extractvalue { i64, i1 } %598, 1
  br i1 %599, label %600, label %593

600:                                              ; preds = %596
  %601 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %587 monotonic, align 8
  %602 = tail call i64 @llvm.ssub.sat.i64(i64 %601, i64 %587)
  %603 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %604

604:                                              ; preds = %607, %600
  %605 = phi i64 [ %603, %600 ], [ %610, %607 ]
  %606 = icmp slt i64 %602, %605
  br i1 %606, label %607, label %611

607:                                              ; preds = %604
  %608 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %605, i64 %602 monotonic monotonic, align 8
  %609 = extractvalue { i64, i1 } %608, 1
  %610 = extractvalue { i64, i1 } %608, 0
  br i1 %609, label %611, label %604

611:                                              ; preds = %607, %604
  %612 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit35)

__rustc::__rust_dealloc (.exit35): ; preds = %593, %611
  tail call void @free(ptr noundef nonnull %439) #92
  br label %624

613:                                              ; preds = %520, %516, %506
  %614 = phi { ptr, i32 } [ %507, %506 ], [ %517, %516 ], [ %517, %520 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %18) #89, !noalias !36648
  br label %487

615:                                              ; preds = %623, %618
  %616 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !36605
  unreachable

617:                                              ; preds = %__rustc::__rust_alloc (.exit33.thread)
  unreachable

618:                                              ; preds = %437, %436, %434
  %619 = phi { ptr, i32 } [ %488, %436 ], [ %438, %437 ], [ %435, %434 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %21) #89
          to label %620 unwind label %615, !noalias !36605

620:                                              ; preds = %618
  %621 = atomicrmw sub ptr %364, i64 1 release, align 8, !noalias !36693
  %622 = icmp eq i64 %621, 1
  br i1 %622, label %623, label %369

623:                                              ; preds = %620
  fence acquire, !noalias !36605
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %19) #91
          to label %369 unwind label %615

624:                                              ; preds = %__rustc::__rust_dealloc (.exit35), %582
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  call void @llvm.lifetime.end.p0(ptr nonnull %21)
; call core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  br label %625

625:                                              ; preds = %683, %624, %357, %354, %350
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  ret void

626:                                              ; preds = %687, %369
  %627 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

628:                                              ; preds = %365
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %25, ptr noundef nonnull align 8 dereferenceable(104) %27, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  %629 = getelementptr inbounds nuw i8, ptr %24, i64 24
  store ptr %366, ptr %629, align 8, !alias.scope !36698
  store i64 0, ptr %24, align 8, !alias.scope !36698
  %630 = getelementptr inbounds nuw i8, ptr %24, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %630, align 8, !alias.scope !36698
  %631 = getelementptr inbounds nuw i8, ptr %24, i64 16
  store i64 0, ptr %631, align 8, !alias.scope !36698
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36701)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36704)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36706)
  %632 = load i64, ptr %25, align 8, !range !2059, !alias.scope !36704, !noalias !36708, !noundef !1740
  %633 = icmp eq i64 %632, -1
  br i1 %633, label %635, label %634

634:                                              ; preds = %628
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %26, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %27)
  br label %637

635:                                              ; preds = %628
  %636 = getelementptr inbounds nuw i8, ptr %26, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %636, ptr noundef nonnull readonly align 8 dereferenceable(32) %24, i64 32, i1 false), !alias.scope !36708, !noalias !36704
  store i64 -1, ptr %26, align 8, !alias.scope !36701, !noalias !36709
  br label %637

637:                                              ; preds = %635, %634
  %638 = getelementptr inbounds nuw i8, ptr %25, i64 72
  %639 = load i64, ptr %638, align 8, !range !1778, !alias.scope !36710, !noalias !36708, !noundef !1740
  %640 = icmp ugt i64 %639, 5
  br i1 %640, label %641, label %675

641:                                              ; preds = %637
  %642 = getelementptr inbounds nuw i8, ptr %25, i64 80
  %643 = load ptr, ptr %642, align 8, !alias.scope !36704, !noalias !36708, !nonnull !1740, !noundef !1740
  %644 = mul i64 %639, 3
  %645 = add i64 %644, -3
  %646 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %647 = load i64, ptr %646, align 8, !noalias !36713, !noundef !1740
  %648 = tail call i64 @llvm.umin.i64(i64 %645, i64 9223372036854775807)
  %649 = tail call i64 @llvm.ssub.sat.i64(i64 %647, i64 %648)
  store i64 %649, ptr %646, align 8, !noalias !36713
  %650 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %651 = load i64, ptr %650, align 8, !noalias !36713, !noundef !1740
  %652 = icmp slt i64 %649, %651
  br i1 %652, label %653, label %.preheader215

653:                                              ; preds = %641
  store i64 %649, ptr %650, align 8, !noalias !36713
  br label %.preheader215

.preheader215:                                    ; preds = %653, %641
  br label %654

654:                                              ; preds = %.preheader215, %657
  %655 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36713
  %656 = icmp slt i64 %655, 0
  br i1 %656, label %657, label %__rustc::__rust_dealloc (.exit36)

657:                                              ; preds = %654
  %658 = add nsw i64 %655, 1
  %659 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %655, i64 %658 acq_rel acquire, align 8, !noalias !36713
  %660 = extractvalue { i64, i1 } %659, 1
  br i1 %660, label %661, label %654

661:                                              ; preds = %657
  %662 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %648 monotonic, align 8, !noalias !36713
  %663 = tail call i64 @llvm.ssub.sat.i64(i64 %662, i64 %648)
  %664 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36713
  br label %665

665:                                              ; preds = %668, %661
  %666 = phi i64 [ %664, %661 ], [ %671, %668 ]
  %667 = icmp slt i64 %663, %666
  br i1 %667, label %668, label %672

668:                                              ; preds = %665
  %669 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %666, i64 %663 monotonic monotonic, align 8, !noalias !36713
  %670 = extractvalue { i64, i1 } %669, 1
  %671 = extractvalue { i64, i1 } %669, 0
  br i1 %670, label %672, label %665

672:                                              ; preds = %668, %665
  %673 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36713
  br label %__rustc::__rust_dealloc (.exit36)

__rustc::__rust_dealloc (.exit36): ; preds = %654, %672
  %674 = icmp ne i64 %645, 0
  tail call void @llvm.assume(i1 %674), !noalias !36713
  tail call void @free(ptr noundef nonnull %643) #92, !noalias !36713
  br label %675

675:                                              ; preds = %__rustc::__rust_dealloc (.exit36), %637
  %676 = getelementptr inbounds nuw i8, ptr %25, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36716)
  %677 = load ptr, ptr %676, align 8, !alias.scope !36719, !noalias !36708, !noundef !1740
  %678 = icmp eq ptr %677, null
  br i1 %678, label %683, label %679

679:                                              ; preds = %675
  %680 = atomicrmw sub ptr %677, i64 1 release, align 8, !noalias !36720
  %681 = icmp eq i64 %680, 1
  br i1 %681, label %682, label %683

682:                                              ; preds = %679
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %676) #91
  br label %683

683:                                              ; preds = %682, %679, %675
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  call void @llvm.lifetime.end.p0(ptr nonnull %25)
  %684 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %684, ptr noundef nonnull align 8 dereferenceable(96) %26, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  br label %625

685:                                              ; preds = %687, %269
  %686 = phi { ptr, i32 } [ %371, %269 ], [ %688, %687 ]
  resume { ptr, i32 } %686

687:                                              ; preds = %270, %269, %259, %234, %218, %88, %84
  %688 = phi { ptr, i32 } [ %271, %270 ], [ %371, %269 ], [ %85, %84 ], [ %85, %88 ], [ %209, %218 ], [ %246, %234 ], [ %260, %259 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %27) #89
          to label %685 unwind label %626
}
