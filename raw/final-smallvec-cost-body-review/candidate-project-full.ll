define void @purrdf_sparql_eval::modifier::eval_project::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(address) %3, i64 noundef range(i64 0, 576460752303423488) %4, ptr noalias nofree noundef align 16 dereferenceable(1248) %5) unnamed_addr #8 personality ptr @rust_eh_personality !guid !35792 {
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
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35793)
  %28 = getelementptr inbounds nuw i8, ptr %5, i64 1120
  %29 = getelementptr inbounds nuw i8, ptr %5, i64 1136
  %30 = load i64, ptr %29, align 16, !alias.scope !35793, !noalias !35796, !noundef !1740
  %31 = icmp ult i64 %30, 192153584101141163
  tail call void @llvm.assume(i1 %31)
  %32 = icmp eq i64 %30, 0
  br i1 %32, label %33, label %34

33:                                               ; preds = %6
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %20, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %272 unwind label %270, !inline_history !35800

34:                                               ; preds = %6
  %35 = getelementptr inbounds nuw i8, ptr %5, i64 1128
  %36 = load ptr, ptr %35, align 8, !alias.scope !35793, !noalias !35796, !nonnull !1740, !noundef !1740
  %37 = mul nuw nsw i64 %30, 48
  %38 = getelementptr inbounds nuw i8, ptr %36, i64 %37
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !35801
  %39 = shl nuw nsw i64 %4, 4
  %40 = getelementptr inbounds nuw i8, ptr %3, i64 %39
  %41 = icmp eq i64 %4, 0
  br i1 %41, label %42, label %.preheader64

42:                                               ; preds = %34
  %43 = getelementptr inbounds nuw i8, ptr %36, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35808)
  %44 = getelementptr i8, ptr %36, i64 24
  %45 = load ptr, ptr %44, align 8, !noalias !35811
  %46 = getelementptr i8, ptr %36, i64 32
  %47 = load i64, ptr %46, align 8, !noalias !35811
  br label %.loopexit63

.preheader64:                                     ; preds = %34, %73
  %48 = phi ptr [ %49, %73 ], [ %36, %34 ]
  %49 = getelementptr inbounds nuw i8, ptr %48, i64 48
  %50 = getelementptr inbounds nuw i8, ptr %48, i64 24
  %51 = load ptr, ptr %50, align 8, !noalias !35814, !nonnull !1740, !noundef !1740
  %52 = getelementptr i8, ptr %48, i64 32
  %53 = load i64, ptr %52, align 8, !noalias !35814
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35808)
  %54 = getelementptr inbounds nuw i8, ptr %51, i64 16
  br label %55

55:                                               ; preds = %71, %.preheader64
  %56 = phi ptr [ %3, %.preheader64 ], [ %57, %71 ]
  %57 = getelementptr inbounds nuw i8, ptr %56, i64 16
  %58 = load ptr, ptr %56, align 8, !alias.scope !35808, !noalias !35820, !nonnull !1740, !noundef !1740
  %59 = getelementptr i8, ptr %56, i64 8
  %60 = load i64, ptr %59, align 8, !alias.scope !35808, !noalias !35820, !noundef !1740
  %61 = icmp eq ptr %58, %51
  %62 = icmp eq i64 %60, %53
  %63 = xor i1 %62, true
  %64 = or i1 %61, %63
  br i1 %64, label %69, label %65

65:                                               ; preds = %55
  %66 = getelementptr inbounds nuw i8, ptr %58, i64 16
  %67 = tail call i32 @bcmp(ptr nonnull readonly %66, ptr nonnull readonly %54, i64 %53), !alias.scope !35823, !noalias !35827
  %68 = icmp eq i32 %67, 0
  br i1 %68, label %73, label %71

69:                                               ; preds = %55
  %70 = and i1 %61, %62
  br i1 %70, label %73, label %71

71:                                               ; preds = %69, %65
  %72 = icmp eq ptr %57, %40
  br i1 %72, label %.loopexit63, label %55

73:                                               ; preds = %69, %65
  %74 = icmp eq ptr %49, %38
  br i1 %74, label %.thread, label %.preheader64

.thread:                                          ; preds = %73
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !35801
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !35828
  store ptr inttoptr (i64 8 to ptr), ptr %12, align 8, !noalias !35828
  %75 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %76 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 0, ptr %76, align 8, !noalias !35828
  %77 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr inttoptr (i64 8 to ptr), ptr %77, align 8, !noalias !35828
  br label %.loopexit56

.loopexit63:                                      ; preds = %71, %42
  %78 = phi i64 [ %47, %42 ], [ %53, %71 ]
  %79 = phi ptr [ %45, %42 ], [ %51, %71 ]
  %80 = phi ptr [ %43, %42 ], [ %49, %71 ]
  %81 = atomicrmw add ptr %79, i64 1 monotonic, align 8, !noalias !35811
  %82 = icmp slt i64 %81, 0
  br i1 %82, label %83, label %89

83:                                               ; preds = %.loopexit63
  tail call void @llvm.trap()
  unreachable

84:                                               ; preds = %__rustc::__rust_alloc (.exit.thread)
  %85 = landingpad { ptr, i32 }
          cleanup
  %86 = atomicrmw sub ptr %79, i64 1 release, align 8, !noalias !35829
  %87 = icmp eq i64 %86, 1
  br i1 %87, label %88, label %814

88:                                               ; preds = %84
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %8) #92
          to label %814 unwind label %219, !noalias !35801

89:                                               ; preds = %.loopexit63
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !35801
  store ptr %79, ptr %8, align 8, !noalias !35801
  %90 = getelementptr inbounds nuw i8, ptr %8, i64 8
  store i64 %78, ptr %90, align 8, !noalias !35801
  %91 = tail call noundef dereferenceable_or_null(64) ptr @malloc(i64 noundef range(i64 1, 0) 64) #93, !noalias !35836
  %92 = icmp eq ptr %91, null
  br i1 %92, label %__rustc::__rust_alloc (.exit.thread), label %93

93:                                               ; preds = %89
  %94 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %95 = load i64, ptr %94, align 8, !noalias !35836, !noundef !1740
  %96 = tail call i64 @llvm.uadd.sat.i64(i64 %95, i64 1)
  store i64 %96, ptr %94, align 8, !noalias !35836
  %97 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %98 = load i64, ptr %97, align 8, !noalias !35836, !noundef !1740
  %99 = tail call i64 @llvm.uadd.sat.i64(i64 %98, i64 64)
  store i64 %99, ptr %97, align 8, !noalias !35836
  %100 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %101 = load i64, ptr %100, align 8, !noalias !35836, !noundef !1740
  %102 = tail call i64 @llvm.sadd.sat.i64(i64 %101, i64 64)
  store i64 %102, ptr %100, align 8, !noalias !35836
  %103 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %104 = load i64, ptr %103, align 8, !noalias !35836, !noundef !1740
  %105 = icmp sgt i64 %102, %104
  br i1 %105, label %106, label %.preheader382

106:                                              ; preds = %93
  store i64 %102, ptr %103, align 8, !noalias !35836
  br label %.preheader382

.preheader382:                                    ; preds = %106, %93
  br label %107

107:                                              ; preds = %.preheader382, %110
  %108 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !35836
  %109 = icmp slt i64 %108, 0
  br i1 %109, label %110, label %__rustc::__rust_alloc (.exit)

110:                                              ; preds = %107
  %111 = add nsw i64 %108, 1
  %112 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %108, i64 %111 acq_rel acquire, align 8, !noalias !35836
  %113 = extractvalue { i64, i1 } %112, 1
  br i1 %113, label %114, label %107

114:                                              ; preds = %110
  %115 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !35836
  %116 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 64 monotonic, align 8, !noalias !35836
  %117 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 64 monotonic, align 8, !noalias !35836
  %118 = tail call i64 @llvm.sadd.sat.i64(i64 %117, i64 64)
  %119 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !35836
  br label %120

120:                                              ; preds = %123, %114
  %121 = phi i64 [ %119, %114 ], [ %126, %123 ]
  %122 = icmp sgt i64 %118, %121
  br i1 %122, label %123, label %127

123:                                              ; preds = %120
  %124 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %121, i64 %118 monotonic monotonic, align 8, !noalias !35836
  %125 = extractvalue { i64, i1 } %124, 1
  %126 = extractvalue { i64, i1 } %124, 0
  br i1 %125, label %127, label %120

127:                                              ; preds = %123, %120
  %128 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !35836
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %89
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 64) #94
          to label %129 unwind label %84, !noalias !35801

129:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %107, %127
  store ptr %79, ptr %91, align 8, !noalias !35801
  %130 = getelementptr inbounds nuw i8, ptr %91, i64 8
  store i64 %78, ptr %130, align 8, !noalias !35801
  store i64 4, ptr %9, align 8, !noalias !35801
  %131 = getelementptr inbounds nuw i8, ptr %9, i64 8
  store ptr %91, ptr %131, align 8, !noalias !35801
  %132 = getelementptr inbounds nuw i8, ptr %9, i64 16
  store i64 1, ptr %132, align 8, !noalias !35801
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !35801
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35839)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35842)
  %133 = icmp eq ptr %80, %38
  br i1 %133, label %.loopexit58, label %134

134:                                              ; preds = %__rustc::__rust_alloc (.exit)
  %135 = getelementptr inbounds nuw i8, ptr %7, i64 8
  br i1 %41, label %.preheader, label %.preheader60

.preheader:                                       ; preds = %134, %153
  %136 = phi ptr [ %154, %153 ], [ %91, %134 ]
  %137 = phi i64 [ %157, %153 ], [ 1, %134 ]
  %138 = phi ptr [ %139, %153 ], [ %80, %134 ]
  %139 = getelementptr inbounds nuw i8, ptr %138, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35845)
  %140 = getelementptr i8, ptr %138, i64 24
  %141 = load ptr, ptr %140, align 8, !noalias !35848, !nonnull !1740, !noundef !1740
  %142 = getelementptr i8, ptr %138, i64 32
  %143 = load i64, ptr %142, align 8, !noalias !35848
  %144 = atomicrmw add ptr %141, i64 1 monotonic, align 8, !noalias !35848
  %145 = icmp slt i64 %144, 0
  br i1 %145, label %.loopexit57, label %146

146:                                              ; preds = %.preheader
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !35853
  store ptr %141, ptr %7, align 8, !noalias !35853
  store i64 %143, ptr %135, align 8, !noalias !35853
  %147 = icmp samesign ult i64 %137, 576460752303423488
  tail call void @llvm.assume(i1 %147)
  %148 = load i64, ptr %9, align 8, !range !1835, !alias.scope !35854, !noalias !35855, !noundef !1740
  %149 = icmp eq i64 %137, %148
  br i1 %149, label %150, label %153

150:                                              ; preds = %146
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %137, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %151 unwind label %159, !noalias !35855

151:                                              ; preds = %150
  %152 = load ptr, ptr %131, align 8, !alias.scope !35854, !noalias !35855
  br label %153

153:                                              ; preds = %151, %146
  %154 = phi ptr [ %152, %151 ], [ %136, %146 ]
  %155 = getelementptr inbounds nuw [16 x i8], ptr %154, i64 %137
  store ptr %141, ptr %155, align 8, !noalias !35853
  %156 = getelementptr inbounds nuw i8, ptr %155, i64 8
  store i64 %143, ptr %156, align 8, !noalias !35853
  %157 = add nuw nsw i64 %137, 1
  store i64 %157, ptr %132, align 8, !alias.scope !35854, !noalias !35855
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !35853
  %158 = icmp eq ptr %139, %38
  br i1 %158, label %.loopexit58, label %.preheader

159:                                              ; preds = %150
  %160 = landingpad { ptr, i32 }
          cleanup
  br label %207

.preheader60:                                     ; preds = %134, %199
  %161 = phi ptr [ %200, %199 ], [ %91, %134 ]
  %162 = phi i64 [ %203, %199 ], [ 1, %134 ]
  %163 = phi ptr [ %166, %199 ], [ %80, %134 ]
  br label %164

164:                                              ; preds = %190, %.preheader60
  %165 = phi ptr [ %166, %190 ], [ %163, %.preheader60 ]
  %166 = getelementptr inbounds nuw i8, ptr %165, i64 48
  %167 = getelementptr i8, ptr %165, i64 24
  %168 = load ptr, ptr %167, align 8, !noalias !35856, !nonnull !1740, !noundef !1740
  %169 = getelementptr i8, ptr %165, i64 32
  %170 = load i64, ptr %169, align 8, !noalias !35856
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35845)
  %171 = getelementptr inbounds nuw i8, ptr %168, i64 16
  br label %172

172:                                              ; preds = %188, %164
  %173 = phi ptr [ %3, %164 ], [ %174, %188 ]
  %174 = getelementptr inbounds nuw i8, ptr %173, i64 16
  %175 = load ptr, ptr %173, align 8, !alias.scope !35845, !noalias !35862, !nonnull !1740, !noundef !1740
  %176 = getelementptr i8, ptr %173, i64 8
  %177 = load i64, ptr %176, align 8, !alias.scope !35845, !noalias !35862, !noundef !1740
  %178 = icmp eq ptr %175, %168
  %179 = icmp eq i64 %177, %170
  %180 = xor i1 %179, true
  %181 = or i1 %178, %180
  br i1 %181, label %186, label %182

182:                                              ; preds = %172
  %183 = getelementptr inbounds nuw i8, ptr %175, i64 16
  %184 = tail call i32 @bcmp(ptr nonnull readonly %183, ptr nonnull readonly %171, i64 %170), !alias.scope !35865, !noalias !35869
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
  br i1 %191, label %.loopexit58, label %164

192:                                              ; preds = %188
  %193 = atomicrmw add ptr %168, i64 1 monotonic, align 8, !noalias !35848
  %194 = icmp slt i64 %193, 0
  br i1 %194, label %.loopexit57, label %195

.loopexit57:                                      ; preds = %192, %.preheader
  tail call void @llvm.trap()
  unreachable

195:                                              ; preds = %192
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !35853
  store ptr %168, ptr %7, align 8, !noalias !35853
  store i64 %170, ptr %135, align 8, !noalias !35853
  %196 = icmp samesign ult i64 %162, 576460752303423488
  tail call void @llvm.assume(i1 %196)
  %197 = load i64, ptr %9, align 8, !range !1835, !alias.scope !35854, !noalias !35855, !noundef !1740
  %198 = icmp eq i64 %162, %197
  br i1 %198, label %213, label %199

199:                                              ; preds = %214, %195
  %200 = phi ptr [ %215, %214 ], [ %161, %195 ]
  %201 = getelementptr inbounds nuw [16 x i8], ptr %200, i64 %162
  store ptr %168, ptr %201, align 8, !noalias !35853
  %202 = getelementptr inbounds nuw i8, ptr %201, i64 8
  store i64 %170, ptr %202, align 8, !noalias !35853
  %203 = add nuw nsw i64 %162, 1
  store i64 %203, ptr %132, align 8, !alias.scope !35854, !noalias !35855
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !35853
  %204 = icmp eq ptr %166, %38
  br i1 %204, label %.loopexit58, label %.preheader60

205:                                              ; preds = %213
  %206 = landingpad { ptr, i32 }
          cleanup
  br label %207

207:                                              ; preds = %205, %159
  %208 = phi ptr [ %168, %205 ], [ %141, %159 ]
  %209 = phi { ptr, i32 } [ %206, %205 ], [ %160, %159 ]
  %210 = atomicrmw sub ptr %208, i64 1 release, align 8, !noalias !35870
  %211 = icmp eq i64 %210, 1
  br i1 %211, label %212, label %218

212:                                              ; preds = %207
  fence acquire
; invoke <alloc::sync::Arc<str>>::drop_slow
  invoke void @<alloc::sync::Arc<str>>::drop_slow(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %7) #92
          to label %218 unwind label %216, !noalias !35853

213:                                              ; preds = %195
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.10720091597982897309)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9, i64 noundef %162, i64 noundef 1, i64 noundef 8, i64 noundef 16)
          to label %214 unwind label %205, !noalias !35855

214:                                              ; preds = %213
  %215 = load ptr, ptr %131, align 8, !alias.scope !35854, !noalias !35855
  br label %199

216:                                              ; preds = %212
  %217 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !35853
  unreachable

218:                                              ; preds = %212, %207
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>
  invoke void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %9) #90
          to label %814 unwind label %219, !noalias !35801

219:                                              ; preds = %218, %88
  %220 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !35801
  unreachable

.loopexit58:                                      ; preds = %199, %190, %153, %__rustc::__rust_alloc (.exit)
  %221 = phi i64 [ %162, %190 ], [ %157, %153 ], [ 1, %__rustc::__rust_alloc (.exit) ], [ %203, %199 ]
  %222 = load i64, ptr %9, align 8, !noalias !35877
  %223 = load ptr, ptr %131, align 8, !noalias !35877
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !35801
  %224 = icmp ult i64 %221, 576460752303423488
  tail call void @llvm.assume(i1 %224)
  %225 = shl nuw nsw i64 %221, 4
  %226 = getelementptr inbounds nuw i8, ptr %223, i64 %225
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !35828
  store ptr %223, ptr %12, align 8, !noalias !35828
  %227 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %228 = getelementptr inbounds nuw i8, ptr %12, i64 16
  store i64 %222, ptr %228, align 8, !noalias !35828
  %229 = getelementptr inbounds nuw i8, ptr %12, i64 24
  store ptr %226, ptr %229, align 8, !noalias !35828
  %230 = getelementptr inbounds nuw i8, ptr %11, i64 24
  %231 = getelementptr inbounds nuw i8, ptr %11, i64 32
  %232 = getelementptr inbounds nuw i8, ptr %11, i64 40
  %233 = load i64, ptr %29, align 16, !alias.scope !35878, !noalias !35881
  br label %235

234:                                              ; preds = %245
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12) #90
          to label %814 unwind label %262, !noalias !35883, !inline_history !35884

235:                                              ; preds = %264, %.loopexit58
  %236 = phi i64 [ %233, %.loopexit58 ], [ %267, %264 ]
  %237 = phi ptr [ %223, %.loopexit58 ], [ %238, %264 ]
  %238 = getelementptr inbounds nuw i8, ptr %237, i64 16
  %239 = load ptr, ptr %237, align 8, !noalias !35885, !nonnull !1740, !noundef !1740
  %240 = getelementptr inbounds nuw i8, ptr %237, i64 8
  %241 = load i64, ptr %240, align 8, !noalias !35885, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !35828
  store ptr %239, ptr %230, align 8, !noalias !35828
  store i64 %241, ptr %231, align 8, !noalias !35828
  store i64 2, ptr %11, align 8, !noalias !35828
  store i8 0, ptr %232, align 8, !noalias !35828
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35878)
  %242 = load i64, ptr %28, align 16, !range !1835, !alias.scope !35878, !noalias !35881, !noundef !1740
  %243 = icmp eq i64 %236, %242
  br i1 %243, label %244, label %264

244:                                              ; preds = %235
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %28)
          to label %264 unwind label %245, !noalias !35881

245:                                              ; preds = %244
  %246 = landingpad { ptr, i32 }
          cleanup
  store ptr %238, ptr %227, align 8
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(48) %11) #90
          to label %234 unwind label %247, !noalias !35888

247:                                              ; preds = %245
  %248 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !35888
  unreachable

.loopexit56:                                      ; preds = %264, %.thread
  %249 = phi ptr [ %75, %.thread ], [ %227, %264 ]
  %250 = phi ptr [ inttoptr (i64 8 to ptr), %.thread ], [ %226, %264 ]
  store ptr %250, ptr %249, align 1
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_algebra::ast::Variable>>(ptr noalias nofree noundef align 8 dereferenceable(32) %12)
          to label %251 unwind label %270, !inline_history !35884

251:                                              ; preds = %.loopexit56
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !35828
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !35828
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %10, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %5, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %252 unwind label %270, !inline_history !35800

252:                                              ; preds = %251
  %253 = load i64, ptr %29, align 16, !alias.scope !35889, !noalias !35892, !noundef !1740
  %254 = icmp ugt i64 %30, %253
  br i1 %254, label %261, label %255

255:                                              ; preds = %252
  %256 = sub nuw i64 %253, %30
  %257 = load ptr, ptr %35, align 8, !alias.scope !35889, !noalias !35892, !nonnull !1740, !noundef !1740
  %258 = getelementptr inbounds nuw [48 x i8], ptr %257, i64 %30
  store i64 %30, ptr %29, align 16, !alias.scope !35889, !noalias !35892
; invoke core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>
  invoke fastcc void @core::ptr::drop_glue::<[purrdf_sparql_eval::service_endpoints::EndpointFrame<purrdf_core::ir::term::TermId>]>(ptr noalias nofree noundef nonnull align 8 %258, i64 noundef %256)
          to label %261 unwind label %259

259:                                              ; preds = %255
  %260 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>
  invoke fastcc void @core::ptr::drop_glue::<core::result::Result<purrdf_sparql_eval::governor::lift::Evaluated<purrdf_core::ir::term::TermId>, purrdf_sparql_eval::error::EvalError>>(ptr noalias nofree noundef align 16 dereferenceable(112) %10) #90
          to label %814 unwind label %262, !noalias !35892, !inline_history !35884

261:                                              ; preds = %255, %252
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(112) %20, ptr noundef nonnull align 16 dereferenceable(112) %10, i64 112, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !35828
  br label %272

262:                                              ; preds = %259, %234
  %263 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !35892, !inline_history !35884
  unreachable

264:                                              ; preds = %244, %235
  %265 = load ptr, ptr %35, align 8, !alias.scope !35878, !noalias !35881, !nonnull !1740, !noundef !1740
  %266 = getelementptr inbounds nuw [48 x i8], ptr %265, i64 %236
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %266, ptr noundef nonnull align 8 dereferenceable(48) %11, i64 48, i1 false), !noalias !35888
  %267 = add i64 %236, 1
  store i64 %267, ptr %29, align 16, !alias.scope !35878, !noalias !35881
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !35828
  %268 = icmp eq ptr %238, %226
  br i1 %268, label %.loopexit56, label %235

269:                                              ; preds = %369
  br i1 %370, label %814, label %812

270:                                              ; preds = %365, %358, %251, %.loopexit56, %33
  %271 = landingpad { ptr, i32 }
          cleanup
  br label %814

272:                                              ; preds = %261, %33
  %273 = load i64, ptr %20, align 16, !range !1739, !noundef !1740
  %274 = trunc nuw i64 %273 to i1
  br i1 %274, label %275, label %358

275:                                              ; preds = %272
  %276 = getelementptr inbounds nuw i8, ptr %20, i64 16
  %277 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %277, ptr noundef nonnull align 16 dereferenceable(96) %276, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35893)
  %278 = getelementptr inbounds nuw i8, ptr %27, i64 72
  %279 = load i64, ptr %278, align 8, !range !1778, !alias.scope !35896, !noundef !1740
  %280 = icmp ugt i64 %279, 5
  br i1 %280, label %281, label %315

281:                                              ; preds = %275
  %282 = getelementptr inbounds nuw i8, ptr %27, i64 80
  %283 = load ptr, ptr %282, align 8, !alias.scope !35893, !nonnull !1740, !noundef !1740
  %284 = mul i64 %279, 3
  %285 = add i64 %284, -3
  %286 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %287 = load i64, ptr %286, align 8, !noalias !35899, !noundef !1740
  %288 = tail call i64 @llvm.umin.i64(i64 %285, i64 9223372036854775807)
  %289 = tail call i64 @llvm.ssub.sat.i64(i64 %287, i64 %288)
  store i64 %289, ptr %286, align 8, !noalias !35899
  %290 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %291 = load i64, ptr %290, align 8, !noalias !35899, !noundef !1740
  %292 = icmp slt i64 %289, %291
  br i1 %292, label %293, label %.preheader334

293:                                              ; preds = %281
  store i64 %289, ptr %290, align 8, !noalias !35899
  br label %.preheader334

.preheader334:                                    ; preds = %293, %281
  br label %294

294:                                              ; preds = %.preheader334, %297
  %295 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !35899
  %296 = icmp slt i64 %295, 0
  br i1 %296, label %297, label %__rustc::__rust_dealloc (.exit)

297:                                              ; preds = %294
  %298 = add nsw i64 %295, 1
  %299 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %295, i64 %298 acq_rel acquire, align 8, !noalias !35899
  %300 = extractvalue { i64, i1 } %299, 1
  br i1 %300, label %301, label %294

301:                                              ; preds = %297
  %302 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %288 monotonic, align 8, !noalias !35899
  %303 = tail call i64 @llvm.ssub.sat.i64(i64 %302, i64 %288)
  %304 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !35899
  br label %305

305:                                              ; preds = %308, %301
  %306 = phi i64 [ %304, %301 ], [ %311, %308 ]
  %307 = icmp slt i64 %303, %306
  br i1 %307, label %308, label %312

308:                                              ; preds = %305
  %309 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %306, i64 %303 monotonic monotonic, align 8, !noalias !35899
  %310 = extractvalue { i64, i1 } %309, 1
  %311 = extractvalue { i64, i1 } %309, 0
  br i1 %310, label %312, label %305

312:                                              ; preds = %308, %305
  %313 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !35899
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %294, %312
  %314 = icmp ne i64 %285, 0
  tail call void @llvm.assume(i1 %314), !noalias !35899
  tail call void @free(ptr noundef nonnull %283) #93, !noalias !35899
  br label %315

315:                                              ; preds = %__rustc::__rust_dealloc (.exit), %275
  %316 = load i64, ptr %27, align 8, !range !2059, !alias.scope !35893, !noundef !1740
  %317 = icmp sgt i64 %316, 0
  br i1 %317, label %318, label %350

318:                                              ; preds = %315
  %319 = getelementptr inbounds nuw i8, ptr %27, i64 8
  %320 = load ptr, ptr %319, align 8, !alias.scope !35893, !nonnull !1740, !noundef !1740
  %321 = mul nuw i64 %316, 3
  %322 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %323 = load i64, ptr %322, align 8, !noalias !35893, !noundef !1740
  %324 = tail call i64 @llvm.umin.i64(i64 %321, i64 9223372036854775807)
  %325 = tail call i64 @llvm.ssub.sat.i64(i64 %323, i64 %324)
  store i64 %325, ptr %322, align 8, !noalias !35893
  %326 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %327 = load i64, ptr %326, align 8, !noalias !35893, !noundef !1740
  %328 = icmp slt i64 %325, %327
  br i1 %328, label %329, label %.preheader333

329:                                              ; preds = %318
  store i64 %325, ptr %326, align 8, !noalias !35893
  br label %.preheader333

.preheader333:                                    ; preds = %329, %318
  br label %330

330:                                              ; preds = %.preheader333, %333
  %331 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !35893
  %332 = icmp slt i64 %331, 0
  br i1 %332, label %333, label %__rustc::__rust_dealloc (.exit48)

333:                                              ; preds = %330
  %334 = add nsw i64 %331, 1
  %335 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %331, i64 %334 acq_rel acquire, align 8, !noalias !35893
  %336 = extractvalue { i64, i1 } %335, 1
  br i1 %336, label %337, label %330

337:                                              ; preds = %333
  %338 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %324 monotonic, align 8, !noalias !35893
  %339 = tail call i64 @llvm.ssub.sat.i64(i64 %338, i64 %324)
  %340 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !35893
  br label %341

341:                                              ; preds = %344, %337
  %342 = phi i64 [ %340, %337 ], [ %347, %344 ]
  %343 = icmp slt i64 %339, %342
  br i1 %343, label %344, label %348

344:                                              ; preds = %341
  %345 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %342, i64 %339 monotonic monotonic, align 8, !noalias !35893
  %346 = extractvalue { i64, i1 } %345, 1
  %347 = extractvalue { i64, i1 } %345, 0
  br i1 %346, label %348, label %341

348:                                              ; preds = %344, %341
  %349 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !35893
  br label %__rustc::__rust_dealloc (.exit48)

__rustc::__rust_dealloc (.exit48): ; preds = %330, %348
  tail call void @free(ptr noundef nonnull %320) #93, !noalias !35893
  br label %350

350:                                              ; preds = %__rustc::__rust_dealloc (.exit48), %315
  %351 = getelementptr inbounds nuw i8, ptr %27, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35902)
  %352 = load ptr, ptr %351, align 8, !alias.scope !35905, !noundef !1740
  %353 = icmp eq ptr %352, null
  br i1 %353, label %752, label %354

354:                                              ; preds = %350
  %355 = atomicrmw sub ptr %352, i64 1 release, align 8, !noalias !35906
  %356 = icmp eq i64 %355, 1
  br i1 %356, label %357, label %752

357:                                              ; preds = %354
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %351) #92
  br label %752

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
          to label %755 unwind label %270

367:                                              ; preds = %363
  %368 = landingpad { ptr, i32 }
          cleanup
  br label %369

369:                                              ; preds = %750, %747, %436, %367
  %370 = phi i1 [ true, %367 ], [ false, %436 ], [ false, %750 ], [ false, %747 ]
  %371 = phi { ptr, i32 } [ %368, %367 ], [ %488, %436 ], [ %746, %750 ], [ %746, %747 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23) #90
          to label %269 unwind label %753

372:                                              ; preds = %363
  call void @llvm.lifetime.start.p0(ptr nonnull %21)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %21, ptr noundef nonnull align 8 dereferenceable(104) %27, i64 104, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35911)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !35914)
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
  store ptr %364, ptr %19, align 8, !noalias !35916
  %373 = getelementptr inbounds nuw i8, ptr %364, i64 24
  %374 = load ptr, ptr %373, align 8, !noalias !35916, !nonnull !1740, !noundef !1740
  %375 = getelementptr inbounds nuw i8, ptr %364, i64 32
  %376 = load i64, ptr %375, align 8, !noalias !35916, !noundef !1740
  %377 = shl nuw i64 %376, 4
  %378 = icmp eq i64 %376, 0
  br i1 %378, label %.loopexit55, label %379

379:                                              ; preds = %372
  %380 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %377) #93, !noalias !35918
  %381 = icmp eq ptr %380, null
  br i1 %381, label %__rustc::__rust_alloc (.exit49.thread), label %382

382:                                              ; preds = %379
  %383 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %384 = load i64, ptr %383, align 8, !noalias !35918, !noundef !1740
  %385 = tail call i64 @llvm.uadd.sat.i64(i64 %384, i64 1)
  store i64 %385, ptr %383, align 8, !noalias !35918
  %386 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %387 = load i64, ptr %386, align 8, !noalias !35918, !noundef !1740
  %388 = tail call i64 @llvm.uadd.sat.i64(i64 %387, i64 %377)
  store i64 %388, ptr %386, align 8, !noalias !35918
  %389 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %390 = load i64, ptr %389, align 8, !noalias !35918, !noundef !1740
  %391 = tail call i64 @llvm.umin.i64(i64 %377, i64 9223372036854775807)
  %392 = tail call i64 @llvm.sadd.sat.i64(i64 %390, i64 %391)
  store i64 %392, ptr %389, align 8, !noalias !35918
  %393 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %394 = load i64, ptr %393, align 8, !noalias !35918, !noundef !1740
  %395 = icmp sgt i64 %392, %394
  br i1 %395, label %396, label %.preheader359

396:                                              ; preds = %382
  store i64 %392, ptr %393, align 8, !noalias !35918
  br label %.preheader359

.preheader359:                                    ; preds = %396, %382
  br label %397

397:                                              ; preds = %.preheader359, %400
  %398 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !35918
  %399 = icmp slt i64 %398, 0
  br i1 %399, label %400, label %__rustc::__rust_alloc (.exit49)

400:                                              ; preds = %397
  %401 = add nsw i64 %398, 1
  %402 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %398, i64 %401 acq_rel acquire, align 8, !noalias !35918
  %403 = extractvalue { i64, i1 } %402, 1
  br i1 %403, label %404, label %397

404:                                              ; preds = %400
  %405 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !35918
  %406 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %377 monotonic, align 8, !noalias !35918
  %407 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %391 monotonic, align 8, !noalias !35918
  %408 = tail call i64 @llvm.sadd.sat.i64(i64 %407, i64 %391)
  %409 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !35918
  br label %410

410:                                              ; preds = %413, %404
  %411 = phi i64 [ %409, %404 ], [ %416, %413 ]
  %412 = icmp sgt i64 %408, %411
  br i1 %412, label %413, label %417

413:                                              ; preds = %410
  %414 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %411, i64 %408 monotonic monotonic, align 8, !noalias !35918
  %415 = extractvalue { i64, i1 } %414, 1
  %416 = extractvalue { i64, i1 } %414, 0
  br i1 %415, label %417, label %410

417:                                              ; preds = %413, %410
  %418 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !35918
  br label %__rustc::__rust_alloc (.exit49)

__rustc::__rust_alloc (.exit49.thread): ; preds = %379
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %377) #94
          to label %419 unwind label %437

419:                                              ; preds = %__rustc::__rust_alloc (.exit49.thread)
  unreachable

__rustc::__rust_alloc (.exit49): ; preds = %397, %417
  %420 = getelementptr inbounds nuw i8, ptr %23, i64 24
  %421 = load ptr, ptr %420, align 8, !noalias !35927, !nonnull !1740, !noundef !1740
  %422 = getelementptr inbounds nuw i8, ptr %421, i64 16
  br label %423

423:                                              ; preds = %427, %__rustc::__rust_alloc (.exit49)
  %424 = phi i64 [ 0, %__rustc::__rust_alloc (.exit49) ], [ %432, %427 ]
  %425 = getelementptr inbounds nuw [16 x i8], ptr %374, i64 %424
; invoke <purrdf_sparql_eval::solution::VarSchema>::index_of
  %426 = invoke fastcc { i64, i64 } @<purrdf_sparql_eval::solution::VarSchema>::index_of (.llvm.10720091597982897309)(ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(56) %422, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %425) #88
          to label %427 unwind label %434, !noalias !35947

427:                                              ; preds = %423
  %428 = extractvalue { i64, i64 } %426, 0
  %429 = extractvalue { i64, i64 } %426, 1
  %430 = getelementptr inbounds nuw [16 x i8], ptr %380, i64 %424
  store i64 %428, ptr %430, align 8, !noalias !35948
  %431 = getelementptr inbounds nuw i8, ptr %430, i64 8
  store i64 %429, ptr %431, align 8, !noalias !35948
  %432 = add nuw i64 %424, 1
  %433 = icmp eq i64 %432, %376
  br i1 %433, label %.loopexit55, label %423

434:                                              ; preds = %423
  %435 = landingpad { ptr, i32 }
          cleanup
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %380, i64 noundef %377, i64 noundef range(i64 1, -9223372036854775807) 8) #93, !noalias !35953
  br label %745

436:                                              ; preds = %490, %487
  br i1 %489, label %369, label %745

437:                                              ; preds = %__rustc::__rust_alloc (.exit49.thread)
  %438 = landingpad { ptr, i32 }
          cleanup
  br label %745

.loopexit55:                                      ; preds = %427, %372
  %439 = phi ptr [ inttoptr (i64 8 to ptr), %372 ], [ %380, %427 ]
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !35916
  %440 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %441 = load i64, ptr %440, align 8, !alias.scope !35914, !noalias !35954, !noundef !1740
  %442 = icmp ult i64 %441, 230584300921369396
  tail call void @llvm.assume(i1 %442)
  %443 = mul nuw nsw i64 %441, 40
  %444 = icmp eq i64 %441, 0
  br i1 %444, label %445, label %448

445:                                              ; preds = %.loopexit55
  store i64 0, ptr %18, align 8, !noalias !35916
  %446 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %446, align 8, !noalias !35916
  %447 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 0, ptr %447, align 8, !noalias !35916
  br label %.loopexit54

448:                                              ; preds = %.loopexit55
  %449 = tail call noundef ptr @malloc(i64 noundef range(i64 1, 0) %443) #93, !noalias !35955
  %450 = icmp eq ptr %449, null
  br i1 %450, label %__rustc::__rust_alloc (.exit50.thread), label %451

451:                                              ; preds = %448
  %452 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %453 = load i64, ptr %452, align 8, !noalias !35955, !noundef !1740
  %454 = tail call i64 @llvm.uadd.sat.i64(i64 %453, i64 1)
  store i64 %454, ptr %452, align 8, !noalias !35955
  %455 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %456 = load i64, ptr %455, align 8, !noalias !35955, !noundef !1740
  %457 = tail call i64 @llvm.uadd.sat.i64(i64 %456, i64 %443)
  store i64 %457, ptr %455, align 8, !noalias !35955
  %458 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %459 = load i64, ptr %458, align 8, !noalias !35955, !noundef !1740
  %460 = tail call i64 @llvm.sadd.sat.i64(i64 %459, i64 %443)
  store i64 %460, ptr %458, align 8, !noalias !35955
  %461 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %462 = load i64, ptr %461, align 8, !noalias !35955, !noundef !1740
  %463 = icmp sgt i64 %460, %462
  br i1 %463, label %464, label %.preheader358

464:                                              ; preds = %451
  store i64 %460, ptr %461, align 8, !noalias !35955
  br label %.preheader358

.preheader358:                                    ; preds = %464, %451
  br label %465

465:                                              ; preds = %.preheader358, %468
  %466 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !35955
  %467 = icmp slt i64 %466, 0
  br i1 %467, label %468, label %__rustc::__rust_alloc (.exit50)

468:                                              ; preds = %465
  %469 = add nsw i64 %466, 1
  %470 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %466, i64 %469 acq_rel acquire, align 8, !noalias !35955
  %471 = extractvalue { i64, i1 } %470, 1
  br i1 %471, label %472, label %465

472:                                              ; preds = %468
  %473 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !35955
  %474 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %443 monotonic, align 8, !noalias !35955
  %475 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %443 monotonic, align 8, !noalias !35955
  %476 = tail call i64 @llvm.sadd.sat.i64(i64 %475, i64 %443)
  %477 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !35955
  br label %478

478:                                              ; preds = %481, %472
  %479 = phi i64 [ %477, %472 ], [ %484, %481 ]
  %480 = icmp sgt i64 %476, %479
  br i1 %480, label %481, label %485

481:                                              ; preds = %478
  %482 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %479, i64 %476 monotonic monotonic, align 8, !noalias !35955
  %483 = extractvalue { i64, i1 } %482, 1
  %484 = extractvalue { i64, i1 } %482, 0
  br i1 %483, label %485, label %478

485:                                              ; preds = %481, %478
  %486 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !35955
  br label %__rustc::__rust_alloc (.exit50)

487:                                              ; preds = %742, %643, %491
  %488 = phi { ptr, i32 } [ %492, %491 ], [ %743, %742 ], [ %644, %643 ]
  %489 = phi i1 [ false, %491 ], [ false, %742 ], [ true, %643 ]
  br i1 %378, label %436, label %490

490:                                              ; preds = %487
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %439, i64 noundef %377, i64 noundef range(i64 1, -9223372036854775807) 8) #93
  br label %436

491:                                              ; preds = %__rustc::__rust_alloc (.exit50.thread)
  %492 = landingpad { ptr, i32 }
          cleanup
  br label %487

__rustc::__rust_alloc (.exit50.thread): ; preds = %448
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef 8, i64 %443) #94
          to label %744 unwind label %491, !noalias !35954

__rustc::__rust_alloc (.exit50): ; preds = %465, %485
  store i64 %441, ptr %18, align 8, !noalias !35916
  %493 = getelementptr inbounds nuw i8, ptr %18, i64 8
  store ptr %449, ptr %493, align 8, !noalias !35916
  %494 = getelementptr inbounds nuw i8, ptr %18, i64 16
  store i64 0, ptr %494, align 8, !noalias !35916
  %495 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %496 = load ptr, ptr %495, align 8, !alias.scope !35914, !noalias !35954, !nonnull !1740, !noundef !1740
  %497 = getelementptr inbounds nuw i8, ptr %496, i64 %443
  %498 = getelementptr inbounds nuw [16 x i8], ptr %439, i64 %376
  %499 = getelementptr inbounds nuw i8, ptr %17, i64 16
  %500 = icmp samesign ugt i64 %376, 4
  %501 = getelementptr inbounds nuw i8, ptr %17, i64 8
  br label %502

502:                                              ; preds = %636, %__rustc::__rust_alloc (.exit50)
  %503 = phi ptr [ %449, %__rustc::__rust_alloc (.exit50) ], [ %637, %636 ]
  %504 = phi i64 [ 0, %__rustc::__rust_alloc (.exit50) ], [ %641, %636 ]
  %505 = phi ptr [ %496, %__rustc::__rust_alloc (.exit50) ], [ %506, %636 ]
  %506 = getelementptr inbounds nuw i8, ptr %505, i64 40
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !35916
  store i64 1, ptr %17, align 8, !noalias !35916
  br i1 %500, label %507, label %511, !prof !1742

507:                                              ; preds = %502
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %17, i64 noundef 0, i64 noundef %376, i1 noundef zeroext true) #92
          to label %508 unwind label %621

508:                                              ; preds = %507
  %509 = load i64, ptr %17, align 8, !range !1778, !alias.scope !35958, !noalias !35961
  %510 = add i64 %509, -1
  br label %511

511:                                              ; preds = %508, %502
  %512 = phi i64 [ %510, %508 ], [ 0, %502 ]
  %513 = icmp ugt i64 %512, 4
  %514 = load ptr, ptr %501, align 8, !alias.scope !35958, !noalias !35961, !nonnull !1740
  %515 = call i64 @llvm.umax.i64(i64 %512, i64 4)
  %516 = select i1 %513, ptr %514, ptr %501
  %517 = select i1 %513, ptr %499, ptr %17
  %518 = load i64, ptr %517, align 8, !alias.scope !35958, !noalias !35961, !noundef !1740
  %519 = add i64 %518, -1
  %520 = icmp ult i64 %519, %515
  br i1 %520, label %521, label %526

521:                                              ; preds = %511
  %522 = getelementptr inbounds nuw i8, ptr %505, i64 8
  %523 = getelementptr inbounds nuw i8, ptr %505, i64 16
  br label %533

524:                                              ; preds = %609
  %525 = add nuw i64 %515, 1
  br label %526

526:                                              ; preds = %524, %511
  %527 = phi ptr [ %439, %511 ], [ %538, %524 ]
  %528 = phi i64 [ %518, %511 ], [ %525, %524 ]
  store i64 %528, ptr %517, align 8
  %529 = icmp eq ptr %527, %498
  br i1 %529, label %.loopexit, label %530

530:                                              ; preds = %526
  %531 = getelementptr inbounds nuw i8, ptr %505, i64 8
  %532 = getelementptr inbounds nuw i8, ptr %505, i64 16
  br label %560

533:                                              ; preds = %609, %521
  %534 = phi i64 [ %519, %521 ], [ %612, %609 ]
  %535 = phi ptr [ %439, %521 ], [ %538, %609 ]
  %536 = icmp eq ptr %535, %498
  br i1 %536, label %614, label %537

537:                                              ; preds = %533
  %538 = getelementptr inbounds nuw i8, ptr %535, i64 16
  %539 = load i64, ptr %535, align 8, !range !1739, !noalias !35963, !noundef !1740
  %540 = getelementptr i8, ptr %535, i64 8
  %541 = load i64, ptr %540, align 8, !noalias !35963
  %542 = trunc nuw i64 %539 to i1
  br i1 %542, label %543, label %609

543:                                              ; preds = %537
  %544 = load i64, ptr %505, align 8, !range !1778, !noalias !35963, !noundef !1740
  %545 = add i64 %544, -1
  %546 = icmp ugt i64 %545, 4
  br i1 %546, label %547, label %551

547:                                              ; preds = %543
  %548 = load ptr, ptr %522, align 8, !noalias !35963, !nonnull !1740, !noundef !1740
  %549 = load i64, ptr %523, align 8, !noalias !35963, !noundef !1740
  %550 = add i64 %549, -1
  br label %551

551:                                              ; preds = %547, %543
  %552 = phi i64 [ %550, %547 ], [ %545, %543 ]
  %553 = phi ptr [ %548, %547 ], [ %522, %543 ]
  %554 = icmp ult i64 %541, %552
  br i1 %554, label %557, label %555

555:                                              ; preds = %551
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %541, i64 noundef %552, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #89
          to label %556 unwind label %616

556:                                              ; preds = %555
  unreachable

557:                                              ; preds = %551
  %558 = getelementptr inbounds nuw [8 x i8], ptr %553, i64 %541
  %559 = load <2 x i32>, ptr %558, align 4, !noalias !35963
  br label %609

560:                                              ; preds = %603, %530
  %561 = phi ptr [ %527, %530 ], [ %562, %603 ]
  %562 = getelementptr inbounds nuw i8, ptr %561, i64 16
  %563 = load i64, ptr %561, align 8, !range !1739, !noalias !35966, !noundef !1740
  %564 = getelementptr i8, ptr %561, i64 8
  %565 = load i64, ptr %564, align 8, !noalias !35966
  %566 = trunc nuw i64 %563 to i1
  br i1 %566, label %567, label %584

567:                                              ; preds = %560
  %568 = load i64, ptr %505, align 8, !range !1778, !noalias !35966, !noundef !1740
  %569 = add i64 %568, -1
  %570 = icmp ugt i64 %569, 4
  br i1 %570, label %571, label %575

571:                                              ; preds = %567
  %572 = load ptr, ptr %531, align 8, !noalias !35966, !nonnull !1740, !noundef !1740
  %573 = load i64, ptr %532, align 8, !noalias !35966, !noundef !1740
  %574 = add i64 %573, -1
  br label %575

575:                                              ; preds = %571, %567
  %576 = phi i64 [ %574, %571 ], [ %569, %567 ]
  %577 = phi ptr [ %572, %571 ], [ %531, %567 ]
  %578 = icmp ult i64 %565, %576
  br i1 %578, label %581, label %579

579:                                              ; preds = %575
; invoke core::panicking::panic_bounds_check
  invoke void @core::panicking::panic_bounds_check(i64 noundef %565, i64 noundef %576, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.791) #89
          to label %580 unwind label %623

580:                                              ; preds = %579
  unreachable

581:                                              ; preds = %575
  %582 = getelementptr inbounds nuw [8 x i8], ptr %577, i64 %565
  %583 = load <2 x i32>, ptr %582, align 4, !noalias !35966
  br label %584

584:                                              ; preds = %581, %560
  %585 = phi <2 x i32> [ <i32 2, i32 undef>, %560 ], [ %583, %581 ]
  %586 = load i64, ptr %17, align 8, !range !1778, !alias.scope !35969, !noundef !1740
  %587 = add i64 %586, -1
  %588 = icmp ugt i64 %587, 4
  %589 = load ptr, ptr %501, align 8, !alias.scope !35969, !nonnull !1740
  %590 = select i1 %588, ptr %589, ptr %501
  %591 = select i1 %588, ptr %499, ptr %17
  %592 = call i64 @llvm.umax.i64(i64 %587, i64 4)
  %593 = load i64, ptr %591, align 8, !alias.scope !35969, !noundef !1740
  %594 = add i64 %593, -1
  %595 = icmp eq i64 %594, %592
  br i1 %595, label %596, label %603, !prof !1742

596:                                              ; preds = %584
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %17, i64 noundef %592, i64 noundef 1, i1 noundef zeroext true) #92
          to label %597 unwind label %619

597:                                              ; preds = %596
  %598 = load i64, ptr %17, align 8, !range !1778, !alias.scope !35969, !noundef !1740
  %599 = icmp ugt i64 %598, 5
  %600 = load ptr, ptr %501, align 8, !alias.scope !35969, !nonnull !1740
  %601 = select i1 %599, ptr %600, ptr %501
  %602 = select i1 %599, ptr %499, ptr %17
  br label %603

603:                                              ; preds = %597, %584
  %604 = phi ptr [ %601, %597 ], [ %590, %584 ]
  %605 = phi ptr [ %602, %597 ], [ %591, %584 ]
  %606 = getelementptr inbounds nuw [8 x i8], ptr %604, i64 %594
  store <2 x i32> %585, ptr %606, align 4
  %607 = add i64 %593, 1
  store i64 %607, ptr %605, align 8, !alias.scope !35969
  %608 = icmp eq ptr %562, %498
  br i1 %608, label %.loopexit, label %560

609:                                              ; preds = %557, %537
  %610 = phi <2 x i32> [ <i32 2, i32 undef>, %537 ], [ %559, %557 ]
  %611 = getelementptr inbounds nuw [8 x i8], ptr %516, i64 %534
  store <2 x i32> %610, ptr %611, align 4, !noalias !35961
  %612 = add i64 %534, 1
  %613 = icmp eq i64 %612, %515
  br i1 %613, label %524, label %533

614:                                              ; preds = %533
  %615 = add nuw i64 %534, 1
  store i64 %615, ptr %517, align 8
  br label %.loopexit

616:                                              ; preds = %555
  %617 = landingpad { ptr, i32 }
          cleanup
  %618 = add nuw i64 %534, 1
  store i64 %618, ptr %517, align 8
  br label %645

619:                                              ; preds = %596
  %620 = landingpad { ptr, i32 }
          cleanup
  br label %645

621:                                              ; preds = %507
  %622 = landingpad { ptr, i32 }
          cleanup
  br label %645

623:                                              ; preds = %579
  %624 = landingpad { ptr, i32 }
          cleanup
  br label %645

.loopexit:                                        ; preds = %603, %614, %526
  call void @llvm.lifetime.start.p0(ptr nonnull %16)
  %625 = load i64, ptr %17, align 8, !noalias !35916
  %626 = load ptr, ptr %501, align 8, !noalias !35916
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %16, ptr noundef nonnull align 8 dereferenceable(24) %499, i64 24, i1 false), !noalias !35916
  call void @llvm.experimental.noalias.scope.decl(metadata !35972)
  %627 = load i64, ptr %18, align 8, !range !1835, !alias.scope !35972, !noalias !35975, !noundef !1740
  %628 = icmp eq i64 %504, %627
  br i1 %628, label %629, label %636

629:                                              ; preds = %.loopexit
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %18)
          to label %630 unwind label %632, !noalias !35975

630:                                              ; preds = %629
  %631 = load ptr, ptr %493, align 8, !alias.scope !35972, !noalias !35975
  br label %636

632:                                              ; preds = %629
  %633 = landingpad { ptr, i32 }
          cleanup
  %634 = icmp ugt i64 %625, 5
  br i1 %634, label %635, label %742

635:                                              ; preds = %632
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %626) ]
  br label %736

636:                                              ; preds = %630, %.loopexit
  %637 = phi ptr [ %631, %630 ], [ %503, %.loopexit ]
  %638 = getelementptr inbounds nuw [40 x i8], ptr %637, i64 %504
  store i64 %625, ptr %638, align 8, !noalias !35977
  %639 = getelementptr inbounds nuw i8, ptr %638, i64 8
  store ptr %626, ptr %639, align 8, !noalias !35977
  %640 = getelementptr inbounds nuw i8, ptr %638, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %640, ptr noundef nonnull align 8 dereferenceable(24) %16, i64 24, i1 false), !noalias !35977
  %641 = add nuw nsw i64 %504, 1
  store i64 %641, ptr %494, align 8, !alias.scope !35972, !noalias !35975
  call void @llvm.lifetime.end.p0(ptr nonnull %16)
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !35916
  %642 = icmp eq ptr %506, %497
  br i1 %642, label %.loopexit54, label %502

643:                                              ; preds = %704
  %644 = landingpad { ptr, i32 }
          cleanup
  br label %487

645:                                              ; preds = %623, %621, %619, %616
  %646 = phi { ptr, i32 } [ %617, %616 ], [ %620, %619 ], [ %622, %621 ], [ %624, %623 ]
  %647 = load i64, ptr %17, align 8, !range !1778, !alias.scope !18581, !noundef !1740
  %648 = icmp ugt i64 %647, 5
  br i1 %648, label %649, label %742

649:                                              ; preds = %645
  %650 = load ptr, ptr %501, align 8, !nonnull !1740, !noundef !1740
  br label %736

651:                                              ; preds = %750, %745
  %652 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91, !noalias !35911
  unreachable

.loopexit54:                                      ; preds = %636, %445
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !35916
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !35916
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %14, ptr noundef nonnull align 8 dereferenceable(104) %21, i64 104, i1 false), !noalias !35978
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !35916
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %13, ptr noundef nonnull align 8 dereferenceable(24) %18, i64 24, i1 false), !noalias !35916
  %653 = getelementptr inbounds nuw i8, ptr %13, i64 24
  store ptr %364, ptr %653, align 8, !noalias !35916
  call void @llvm.experimental.noalias.scope.decl(metadata !35979)
  call void @llvm.experimental.noalias.scope.decl(metadata !35982)
  call void @llvm.experimental.noalias.scope.decl(metadata !35984)
  %654 = load i64, ptr %14, align 8, !range !2059, !alias.scope !35982, !noalias !35986, !noundef !1740
  %655 = icmp eq i64 %654, -1
  br i1 %655, label %657, label %656

656:                                              ; preds = %.loopexit54
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %15, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %13, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %21), !noalias !35911
  br label %659

657:                                              ; preds = %.loopexit54
  %658 = getelementptr inbounds nuw i8, ptr %15, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %658, ptr noundef nonnull readonly align 8 dereferenceable(32) %13, i64 32, i1 false), !alias.scope !35987, !noalias !35988
  store i64 -1, ptr %15, align 8, !alias.scope !35979, !noalias !35989
  br label %659

659:                                              ; preds = %657, %656
  %660 = getelementptr inbounds nuw i8, ptr %14, i64 72
  %661 = load i64, ptr %660, align 8, !range !1778, !alias.scope !35990, !noalias !35986, !noundef !1740
  %662 = icmp ugt i64 %661, 5
  br i1 %662, label %663, label %697

663:                                              ; preds = %659
  %664 = getelementptr inbounds nuw i8, ptr %14, i64 80
  %665 = load ptr, ptr %664, align 8, !alias.scope !35982, !noalias !35986, !nonnull !1740, !noundef !1740
  %666 = mul i64 %661, 3
  %667 = add i64 %666, -3
  %668 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %669 = load i64, ptr %668, align 8, !noalias !35993, !noundef !1740
  %670 = call i64 @llvm.umin.i64(i64 %667, i64 9223372036854775807)
  %671 = call i64 @llvm.ssub.sat.i64(i64 %669, i64 %670)
  store i64 %671, ptr %668, align 8, !noalias !35993
  %672 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %673 = load i64, ptr %672, align 8, !noalias !35993, !noundef !1740
  %674 = icmp slt i64 %671, %673
  br i1 %674, label %675, label %.preheader337

675:                                              ; preds = %663
  store i64 %671, ptr %672, align 8, !noalias !35993
  br label %.preheader337

.preheader337:                                    ; preds = %675, %663
  br label %676

676:                                              ; preds = %.preheader337, %679
  %677 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !35993
  %678 = icmp slt i64 %677, 0
  br i1 %678, label %679, label %__rustc::__rust_dealloc (.exit51)

679:                                              ; preds = %676
  %680 = add nsw i64 %677, 1
  %681 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %677, i64 %680 acq_rel acquire, align 8, !noalias !35993
  %682 = extractvalue { i64, i1 } %681, 1
  br i1 %682, label %683, label %676

683:                                              ; preds = %679
  %684 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %670 monotonic, align 8, !noalias !35993
  %685 = call i64 @llvm.ssub.sat.i64(i64 %684, i64 %670)
  %686 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !35993
  br label %687

687:                                              ; preds = %690, %683
  %688 = phi i64 [ %686, %683 ], [ %693, %690 ]
  %689 = icmp slt i64 %685, %688
  br i1 %689, label %690, label %694

690:                                              ; preds = %687
  %691 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %688, i64 %685 monotonic monotonic, align 8, !noalias !35993
  %692 = extractvalue { i64, i1 } %691, 1
  %693 = extractvalue { i64, i1 } %691, 0
  br i1 %692, label %694, label %687

694:                                              ; preds = %690, %687
  %695 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !35993
  br label %__rustc::__rust_dealloc (.exit51)

__rustc::__rust_dealloc (.exit51): ; preds = %676, %694
  %696 = icmp ne i64 %667, 0
  call void @llvm.assume(i1 %696), !noalias !35993
  call void @free(ptr noundef nonnull %665) #93, !noalias !35993
  br label %697

697:                                              ; preds = %__rustc::__rust_dealloc (.exit51), %659
  %698 = getelementptr inbounds nuw i8, ptr %14, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !35996), !noalias !35954
  %699 = load ptr, ptr %698, align 8, !alias.scope !35999, !noalias !35986, !noundef !1740
  %700 = icmp eq ptr %699, null
  br i1 %700, label %705, label %701

701:                                              ; preds = %697
  %702 = atomicrmw sub ptr %699, i64 1 release, align 8, !noalias !36000
  %703 = icmp eq i64 %702, 1
  br i1 %703, label %704, label %705

704:                                              ; preds = %701
  fence acquire, !noalias !35954
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %698) #92
          to label %705 unwind label %643

705:                                              ; preds = %704, %701, %697
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !35916
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !35916
  %706 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %706, ptr noundef nonnull align 8 dereferenceable(96) %15, i64 96, i1 false), !noalias !36005
  store i64 0, ptr %0, align 16, !alias.scope !35911, !noalias !36005
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !35916
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !35916
  br i1 %378, label %751, label %707

707:                                              ; preds = %705
  %708 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %709 = load i64, ptr %708, align 8, !noundef !1740
  %710 = call i64 @llvm.umin.i64(i64 %377, i64 9223372036854775807)
  %711 = call i64 @llvm.ssub.sat.i64(i64 %709, i64 %710)
  store i64 %711, ptr %708, align 8
  %712 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %713 = load i64, ptr %712, align 8, !noundef !1740
  %714 = icmp slt i64 %711, %713
  br i1 %714, label %715, label %.preheader336

715:                                              ; preds = %707
  store i64 %711, ptr %712, align 8
  br label %.preheader336

.preheader336:                                    ; preds = %715, %707
  br label %716

716:                                              ; preds = %.preheader336, %719
  %717 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %718 = icmp slt i64 %717, 0
  br i1 %718, label %719, label %__rustc::__rust_dealloc (.exit52)

719:                                              ; preds = %716
  %720 = add nsw i64 %717, 1
  %721 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %717, i64 %720 acq_rel acquire, align 8
  %722 = extractvalue { i64, i1 } %721, 1
  br i1 %722, label %723, label %716

723:                                              ; preds = %719
  %724 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %710 monotonic, align 8
  %725 = call i64 @llvm.ssub.sat.i64(i64 %724, i64 %710)
  %726 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %727

727:                                              ; preds = %730, %723
  %728 = phi i64 [ %726, %723 ], [ %733, %730 ]
  %729 = icmp slt i64 %725, %728
  br i1 %729, label %730, label %734

730:                                              ; preds = %727
  %731 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %728, i64 %725 monotonic monotonic, align 8
  %732 = extractvalue { i64, i1 } %731, 1
  %733 = extractvalue { i64, i1 } %731, 0
  br i1 %732, label %734, label %727

734:                                              ; preds = %730, %727
  %735 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit52)

__rustc::__rust_dealloc (.exit52): ; preds = %716, %734
  call void @free(ptr noundef nonnull %439) #93
  br label %751

736:                                              ; preds = %649, %635
  %737 = phi i64 [ %625, %635 ], [ %647, %649 ]
  %738 = phi ptr [ %626, %635 ], [ %650, %649 ]
  %739 = phi { ptr, i32 } [ %633, %635 ], [ %646, %649 ]
  %740 = shl i64 %737, 3
  %741 = add i64 %740, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %738, i64 noundef %741, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !1740
  br label %742

742:                                              ; preds = %736, %645, %632
  %743 = phi { ptr, i32 } [ %646, %645 ], [ %633, %632 ], [ %739, %736 ]
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %18) #90, !noalias !35954
  br label %487

744:                                              ; preds = %__rustc::__rust_alloc (.exit50.thread)
  unreachable

745:                                              ; preds = %437, %436, %434
  %746 = phi { ptr, i32 } [ %488, %436 ], [ %438, %437 ], [ %435, %434 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef nonnull align 8 dereferenceable(104) %21) #90
          to label %747 unwind label %651, !noalias !35911

747:                                              ; preds = %745
  %748 = atomicrmw sub ptr %364, i64 1 release, align 8, !noalias !36006
  %749 = icmp eq i64 %748, 1
  br i1 %749, label %750, label %369

750:                                              ; preds = %747
  fence acquire, !noalias !35911
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %19) #92
          to label %369 unwind label %651

751:                                              ; preds = %__rustc::__rust_dealloc (.exit52), %705
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  call void @llvm.lifetime.end.p0(ptr nonnull %21)
; call core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>
  call fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::solution::SolutionSeq>(ptr noalias nofree noundef align 8 dereferenceable(32) %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  br label %752

752:                                              ; preds = %810, %751, %357, %354, %350
  call void @llvm.lifetime.end.p0(ptr nonnull %27)
  ret void

753:                                              ; preds = %814, %369
  %754 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #91
  unreachable

755:                                              ; preds = %365
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %25, ptr noundef nonnull align 8 dereferenceable(104) %27, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  %756 = getelementptr inbounds nuw i8, ptr %24, i64 24
  store ptr %366, ptr %756, align 8, !alias.scope !36011
  store i64 0, ptr %24, align 8, !alias.scope !36011
  %757 = getelementptr inbounds nuw i8, ptr %24, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %757, align 8, !alias.scope !36011
  %758 = getelementptr inbounds nuw i8, ptr %24, i64 16
  store i64 0, ptr %758, align 8, !alias.scope !36011
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36014)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36017)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36019)
  %759 = load i64, ptr %25, align 8, !range !2059, !alias.scope !36017, !noalias !36021, !noundef !1740
  %760 = icmp eq i64 %759, -1
  br i1 %760, label %762, label %761

761:                                              ; preds = %755
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %26, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %24, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %27)
  br label %764

762:                                              ; preds = %755
  %763 = getelementptr inbounds nuw i8, ptr %26, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %763, ptr noundef nonnull readonly align 8 dereferenceable(32) %24, i64 32, i1 false), !alias.scope !36021, !noalias !36017
  store i64 -1, ptr %26, align 8, !alias.scope !36014, !noalias !36022
  br label %764

764:                                              ; preds = %762, %761
  %765 = getelementptr inbounds nuw i8, ptr %25, i64 72
  %766 = load i64, ptr %765, align 8, !range !1778, !alias.scope !36023, !noalias !36021, !noundef !1740
  %767 = icmp ugt i64 %766, 5
  br i1 %767, label %768, label %802

768:                                              ; preds = %764
  %769 = getelementptr inbounds nuw i8, ptr %25, i64 80
  %770 = load ptr, ptr %769, align 8, !alias.scope !36017, !noalias !36021, !nonnull !1740, !noundef !1740
  %771 = mul i64 %766, 3
  %772 = add i64 %771, -3
  %773 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %774 = load i64, ptr %773, align 8, !noalias !36026, !noundef !1740
  %775 = tail call i64 @llvm.umin.i64(i64 %772, i64 9223372036854775807)
  %776 = tail call i64 @llvm.ssub.sat.i64(i64 %774, i64 %775)
  store i64 %776, ptr %773, align 8, !noalias !36026
  %777 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %778 = load i64, ptr %777, align 8, !noalias !36026, !noundef !1740
  %779 = icmp slt i64 %776, %778
  br i1 %779, label %780, label %.preheader335

780:                                              ; preds = %768
  store i64 %776, ptr %777, align 8, !noalias !36026
  br label %.preheader335

.preheader335:                                    ; preds = %780, %768
  br label %781

781:                                              ; preds = %.preheader335, %784
  %782 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !36026
  %783 = icmp slt i64 %782, 0
  br i1 %783, label %784, label %__rustc::__rust_dealloc (.exit53)

784:                                              ; preds = %781
  %785 = add nsw i64 %782, 1
  %786 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %782, i64 %785 acq_rel acquire, align 8, !noalias !36026
  %787 = extractvalue { i64, i1 } %786, 1
  br i1 %787, label %788, label %781

788:                                              ; preds = %784
  %789 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %775 monotonic, align 8, !noalias !36026
  %790 = tail call i64 @llvm.ssub.sat.i64(i64 %789, i64 %775)
  %791 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !36026
  br label %792

792:                                              ; preds = %795, %788
  %793 = phi i64 [ %791, %788 ], [ %798, %795 ]
  %794 = icmp slt i64 %790, %793
  br i1 %794, label %795, label %799

795:                                              ; preds = %792
  %796 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %793, i64 %790 monotonic monotonic, align 8, !noalias !36026
  %797 = extractvalue { i64, i1 } %796, 1
  %798 = extractvalue { i64, i1 } %796, 0
  br i1 %797, label %799, label %792

799:                                              ; preds = %795, %792
  %800 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !36026
  br label %__rustc::__rust_dealloc (.exit53)

__rustc::__rust_dealloc (.exit53): ; preds = %781, %799
  %801 = icmp ne i64 %772, 0
  tail call void @llvm.assume(i1 %801), !noalias !36026
  tail call void @free(ptr noundef nonnull %770) #93, !noalias !36026
  br label %802

802:                                              ; preds = %__rustc::__rust_dealloc (.exit53), %764
  %803 = getelementptr inbounds nuw i8, ptr %25, i64 96
  tail call void @llvm.experimental.noalias.scope.decl(metadata !36029)
  %804 = load ptr, ptr %803, align 8, !alias.scope !36032, !noalias !36021, !noundef !1740
  %805 = icmp eq ptr %804, null
  br i1 %805, label %810, label %806

806:                                              ; preds = %802
  %807 = atomicrmw sub ptr %804, i64 1 release, align 8, !noalias !36033
  %808 = icmp eq i64 %807, 1
  br i1 %808, label %809, label %810

809:                                              ; preds = %806
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %803) #92
  br label %810

810:                                              ; preds = %809, %806, %802
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  call void @llvm.lifetime.end.p0(ptr nonnull %25)
  %811 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %811, ptr noundef nonnull align 8 dereferenceable(96) %26, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %26)
  br label %752

812:                                              ; preds = %814, %269
  %813 = phi { ptr, i32 } [ %371, %269 ], [ %815, %814 ]
  resume { ptr, i32 } %813

814:                                              ; preds = %270, %269, %259, %234, %218, %88, %84
  %815 = phi { ptr, i32 } [ %271, %270 ], [ %371, %269 ], [ %85, %84 ], [ %85, %88 ], [ %209, %218 ], [ %246, %234 ], [ %260, %259 ]
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %27) #90
          to label %812 unwind label %753
}
