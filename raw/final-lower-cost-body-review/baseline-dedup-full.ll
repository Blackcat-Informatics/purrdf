define void @purrdf_sparql_eval::modifier::eval_dedup::<purrdf_core::ir::dataset::RdfDataset>(ptr dead_on_unwind noalias nofree noundef writable writeonly sret([112 x i8]) align 16 captures(none) dereferenceable(112) %0, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef align 16 dereferenceable(1248) %3) unnamed_addr #10 personality ptr @rust_eh_personality !guid !44285 {
  %5 = alloca [24 x i8], align 8
  %6 = alloca [40 x i8], align 8
  %7 = alloca [24 x i8], align 8
  %8 = alloca [64 x i8], align 8
  %9 = alloca [24 x i8], align 8
  %10 = alloca [24 x i8], align 8
  %11 = alloca [40 x i8], align 8
  %12 = alloca [32 x i8], align 8
  %13 = alloca [32 x i8], align 8
  %14 = alloca [8 x i8], align 8
  %15 = alloca [24 x i8], align 8
  %16 = alloca [40 x i8], align 8
  %17 = alloca [32 x i8], align 8
  %18 = alloca [112 x i8], align 16
  %19 = alloca [32 x i8], align 8
  %20 = alloca [32 x i8], align 8
  %21 = alloca [104 x i8], align 8
  %22 = alloca [96 x i8], align 8
  %23 = alloca [104 x i8], align 8
  %24 = alloca [32 x i8], align 8
  %25 = alloca [32 x i8], align 8
  %26 = alloca [104 x i8], align 8
; call <purrdf_sparql_eval::governor::lift::Lift>::at
  call void @<purrdf_sparql_eval::governor::lift::Lift>::at(ptr noalias nofree noundef nonnull sret([104 x i8]) align 8 captures(none) dereferenceable(104) %26, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %1)
  call void @llvm.lifetime.start.p0(ptr nonnull %24)
  call void @llvm.lifetime.start.p0(ptr nonnull %23)
; invoke purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>
  invoke fastcc void @purrdf_sparql_eval::eval::eval_evaluated_with::<purrdf_core::ir::dataset::RdfDataset, purrdf_sparql_eval::eval::eval_evaluated<purrdf_core::ir::dataset::RdfDataset>::{closure#0}>(ptr noalias nofree noundef nonnull align 16 captures(none) dereferenceable(112) %18, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(144) %2, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %3, ptr noundef nonnull readonly align 8 dereferenceable(144) %2)
          to label %27 unwind label %1450, !inline_history !24788

27:                                               ; preds = %4
  %28 = load i64, ptr %18, align 16, !range !1855, !noundef !1708
  %29 = trunc nuw i64 %28 to i1
  br i1 %29, label %30, label %113

30:                                               ; preds = %27
  %31 = getelementptr inbounds nuw i8, ptr %18, i64 16
  %32 = getelementptr inbounds nuw i8, ptr %0, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(96) %32, ptr noundef nonnull align 16 dereferenceable(96) %31, i64 96, i1 false)
  store i64 1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  %33 = getelementptr inbounds nuw i8, ptr %26, i64 72
  %34 = load i64, ptr %33, align 8, !range !1940, !noundef !1708
  %35 = icmp ugt i64 %34, 5
  br i1 %35, label %36, label %70

36:                                               ; preds = %30
  %37 = getelementptr inbounds nuw i8, ptr %26, i64 80
  %38 = load ptr, ptr %37, align 8, !nonnull !1708, !noundef !1708
  %39 = mul i64 %34, 3
  %40 = add i64 %39, -3
  %41 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %42 = load i64, ptr %41, align 8, !noalias !44286, !noundef !1708
  %43 = tail call i64 @llvm.umin.i64(i64 %40, i64 9223372036854775807)
  %44 = tail call i64 @llvm.ssub.sat.i64(i64 %42, i64 %43)
  store i64 %44, ptr %41, align 8, !noalias !44286
  %45 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %46 = load i64, ptr %45, align 8, !noalias !44286, !noundef !1708
  %47 = icmp slt i64 %44, %46
  br i1 %47, label %48, label %.preheader480

48:                                               ; preds = %36
  store i64 %44, ptr %45, align 8, !noalias !44286
  br label %.preheader480

.preheader480:                                    ; preds = %48, %36
  br label %49

49:                                               ; preds = %.preheader480, %52
  %50 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44286
  %51 = icmp slt i64 %50, 0
  br i1 %51, label %52, label %__rustc::__rust_dealloc (.exit)

52:                                               ; preds = %49
  %53 = add nsw i64 %50, 1
  %54 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %50, i64 %53 acq_rel acquire, align 8, !noalias !44286
  %55 = extractvalue { i64, i1 } %54, 1
  br i1 %55, label %56, label %49

56:                                               ; preds = %52
  %57 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %43 monotonic, align 8, !noalias !44286
  %58 = tail call i64 @llvm.ssub.sat.i64(i64 %57, i64 %43)
  %59 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44286
  br label %60

60:                                               ; preds = %63, %56
  %61 = phi i64 [ %59, %56 ], [ %66, %63 ]
  %62 = icmp slt i64 %58, %61
  br i1 %62, label %63, label %67

63:                                               ; preds = %60
  %64 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %61, i64 %58 monotonic monotonic, align 8, !noalias !44286
  %65 = extractvalue { i64, i1 } %64, 1
  %66 = extractvalue { i64, i1 } %64, 0
  br i1 %65, label %67, label %60

67:                                               ; preds = %63, %60
  %68 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44286
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %49, %67
  %69 = icmp ne i64 %40, 0
  tail call void @llvm.assume(i1 %69), !noalias !44286
  tail call void @free(ptr noundef nonnull %38) #88, !noalias !44286
  br label %70

70:                                               ; preds = %__rustc::__rust_dealloc (.exit), %30
  %71 = load i64, ptr %26, align 8, !range !2062, !noundef !1708
  %72 = icmp sgt i64 %71, 0
  br i1 %72, label %73, label %105

73:                                               ; preds = %70
  %74 = getelementptr inbounds nuw i8, ptr %26, i64 8
  %75 = load ptr, ptr %74, align 8, !nonnull !1708, !noundef !1708
  %76 = mul nuw i64 %71, 3
  %77 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %78 = load i64, ptr %77, align 8, !noalias !44291, !noundef !1708
  %79 = tail call i64 @llvm.umin.i64(i64 %76, i64 9223372036854775807)
  %80 = tail call i64 @llvm.ssub.sat.i64(i64 %78, i64 %79)
  store i64 %80, ptr %77, align 8, !noalias !44291
  %81 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %82 = load i64, ptr %81, align 8, !noalias !44291, !noundef !1708
  %83 = icmp slt i64 %80, %82
  br i1 %83, label %84, label %.preheader479

84:                                               ; preds = %73
  store i64 %80, ptr %81, align 8, !noalias !44291
  br label %.preheader479

.preheader479:                                    ; preds = %84, %73
  br label %85

85:                                               ; preds = %.preheader479, %88
  %86 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44291
  %87 = icmp slt i64 %86, 0
  br i1 %87, label %88, label %__rustc::__rust_dealloc (.exit66)

88:                                               ; preds = %85
  %89 = add nsw i64 %86, 1
  %90 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %86, i64 %89 acq_rel acquire, align 8, !noalias !44291
  %91 = extractvalue { i64, i1 } %90, 1
  br i1 %91, label %92, label %85

92:                                               ; preds = %88
  %93 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %79 monotonic, align 8, !noalias !44291
  %94 = tail call i64 @llvm.ssub.sat.i64(i64 %93, i64 %79)
  %95 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44291
  br label %96

96:                                               ; preds = %99, %92
  %97 = phi i64 [ %95, %92 ], [ %102, %99 ]
  %98 = icmp slt i64 %94, %97
  br i1 %98, label %99, label %103

99:                                               ; preds = %96
  %100 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %97, i64 %94 monotonic monotonic, align 8, !noalias !44291
  %101 = extractvalue { i64, i1 } %100, 1
  %102 = extractvalue { i64, i1 } %100, 0
  br i1 %101, label %103, label %96

103:                                              ; preds = %99, %96
  %104 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44291
  br label %__rustc::__rust_dealloc (.exit66)

__rustc::__rust_dealloc (.exit66): ; preds = %85, %103
  tail call void @free(ptr noundef nonnull %75) #88, !noalias !44291
  br label %105

105:                                              ; preds = %__rustc::__rust_dealloc (.exit66), %70
  %106 = getelementptr inbounds nuw i8, ptr %26, i64 96
  %107 = load ptr, ptr %106, align 8, !noundef !1708
  %108 = icmp eq ptr %107, null
  br i1 %108, label %1363, label %109

109:                                              ; preds = %105
  %110 = atomicrmw sub ptr %107, i64 1 release, align 8, !noalias !44292
  %111 = icmp eq i64 %110, 1
  br i1 %111, label %112, label %1363

112:                                              ; preds = %109
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %106) #87
  br label %1363

113:                                              ; preds = %27
  %114 = getelementptr inbounds nuw i8, ptr %18, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %23, ptr noundef nonnull align 8 dereferenceable(96) %114, i64 96, i1 false)
; invoke <purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>
  invoke fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::absorb::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %24, ptr noalias nofree noundef align 8 dereferenceable(104) %26, i64 noundef 0, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(96) %23)
          to label %115 unwind label %1450

115:                                              ; preds = %113
  %116 = load i64, ptr %24, align 8, !range !2062, !noundef !1708
  %117 = icmp eq i64 %116, -1
  br i1 %117, label %121, label %118

118:                                              ; preds = %115
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %25, ptr noundef nonnull align 8 dereferenceable(32) %24, i64 32, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  call void @llvm.lifetime.start.p0(ptr nonnull %22)
  call void @llvm.lifetime.start.p0(ptr nonnull %21)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(104) %21, ptr noundef nonnull align 8 dereferenceable(104) %26, i64 104, i1 false)
  call void @llvm.lifetime.start.p0(ptr nonnull %20)
  call void @llvm.lifetime.start.p0(ptr nonnull %19)
; invoke purrdf_sparql_eval::blank_scope::without_joined_blanks::<purrdf_core::ir::term::TermId>
  invoke fastcc void @purrdf_sparql_eval::blank_scope::without_joined_blanks::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(none) dereferenceable(32) %19, ptr noalias nofree noundef align 8 captures(address) dereferenceable(32) %25)
          to label %123 unwind label %119

119:                                              ; preds = %118
  %120 = landingpad { ptr, i32 }
          cleanup
  br label %1364

121:                                              ; preds = %115
  call void @llvm.lifetime.end.p0(ptr nonnull %23)
  call void @llvm.lifetime.end.p0(ptr nonnull %24)
  %122 = getelementptr inbounds nuw i8, ptr %0, i64 8
; call <purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>
  call fastcc void @<purrdf_sparql_eval::governor::lift::Lift>::withheld::<purrdf_core::ir::term::TermId>(ptr noalias nofree noundef align 8 captures(address) dereferenceable(96) %122, ptr noalias nofree noundef align 8 captures(address) dereferenceable(104) %26)
  store i64 0, ptr %0, align 16
  br label %1363

123:                                              ; preds = %118
  tail call void @llvm.experimental.noalias.scope.decl(metadata !44299)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !44302)
  call void @llvm.lifetime.start.p0(ptr nonnull %11)
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !44304
  %124 = getelementptr inbounds nuw i8, ptr %19, i64 16
  %125 = load i64, ptr %124, align 8, !alias.scope !44302, !noalias !44299, !noundef !1708
  %126 = icmp ult i64 %125, 230584300921369396
  tail call void @llvm.assume(i1 %126)
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !44304
; invoke <hashbrown::raw::RawTableInner>::fallible_with_capacity::<alloc::alloc::Global>
  invoke fastcc void @<hashbrown::raw::RawTableInner>::fallible_with_capacity::<alloc::alloc::Global>(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(32) %12, i64 noundef 48, i64 noundef range(i64 0, 230584300921369396) %125, i1 noundef zeroext true) #91
          to label %129 unwind label %127

127:                                              ; preds = %123
  %128 = landingpad { ptr, i32 }
          cleanup
  br label %1296

129:                                              ; preds = %123
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %17, ptr noundef nonnull align 8 dereferenceable(32) %12, i64 32, i1 false), !noalias !44304
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !44304
  %130 = getelementptr inbounds nuw i8, ptr %19, i64 8
  %131 = load ptr, ptr %130, align 8, !alias.scope !44302, !noalias !44299, !nonnull !1708, !noundef !1708
  %132 = load i64, ptr %19, align 8, !range !1817, !alias.scope !44302, !noalias !44299, !noundef !1708
  %133 = mul nuw nsw i64 %125, 40
  %134 = getelementptr inbounds nuw i8, ptr %131, i64 %133
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !44304
  store ptr %131, ptr %16, align 8, !noalias !44304
  %135 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %136 = getelementptr inbounds nuw i8, ptr %16, i64 16
  store i64 %132, ptr %136, align 8, !noalias !44304
  %137 = getelementptr inbounds nuw i8, ptr %16, i64 24
  store ptr %134, ptr %137, align 8, !noalias !44304
  %138 = getelementptr inbounds nuw i8, ptr %16, i64 32
  %139 = icmp eq i64 %125, 0
  br i1 %139, label %.loopexit120, label %140

140:                                              ; preds = %129
  %141 = getelementptr inbounds nuw i8, ptr %11, i64 8
  %142 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %143 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %144 = getelementptr inbounds nuw i8, ptr %17, i64 16
  %145 = getelementptr inbounds nuw i8, ptr %11, i64 24
  %146 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %147 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %148

148:                                              ; preds = %1273, %140
  %149 = phi ptr [ %131, %140 ], [ %151, %1273 ]
  %150 = phi i64 [ 0, %140 ], [ %157, %1273 ]
  %151 = getelementptr inbounds nuw i8, ptr %149, i64 40
  %152 = load i64, ptr %149, align 8, !noalias !44305
  %153 = icmp eq i64 %152, 0
  br i1 %153, label %.loopexit120, label %155

154:                                              ; preds = %301, %298
; call core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>
  call fastcc void @core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>, <purrdf_sparql_eval::bgp::BgpProjection>::apply<purrdf_core::ir::term::TermId>::{closure#0}>>(ptr noalias nofree noundef align 8 dereferenceable(40) %16) #89, !noalias !44304
; call core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>>
  call fastcc void @core::ptr::drop_glue::<std::collections::hash::map::HashMap<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>>(ptr noalias nofree noundef align 8 dereferenceable(32) %17) #89, !noalias !44304
  br label %1296

155:                                              ; preds = %148
  %156 = getelementptr inbounds nuw i8, ptr %149, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %141, ptr noundef nonnull align 8 dereferenceable(32) %156, i64 32, i1 false), !noalias !44304
  %157 = add nuw nsw i64 %150, 1
  store i64 %152, ptr %11, align 8, !noalias !44304
  call void @llvm.experimental.noalias.scope.decl(metadata !44311)
  %158 = add i64 %152, -1
  %159 = icmp ugt i64 %158, 4
  %160 = load ptr, ptr %141, align 8, !noalias !44304
  %161 = load i64, ptr %142, align 8, !noalias !44304
  %162 = add i64 %161, -1
  %163 = select i1 %159, i64 %162, i64 %158
  %164 = select i1 %159, ptr %160, ptr %141
  %165 = xor i64 %163, 2746377873070565055
  %166 = insertelement <2 x i64> <i64 poison, i64 8385202752464708517>, i64 %165, i64 0
  %167 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %166, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %168 = shl i64 %163, 3
  %169 = getelementptr inbounds nuw i8, ptr %164, i64 %168
  %170 = icmp eq i64 %163, 0
  %171 = inttoptr i64 %161 to ptr
  br i1 %170, label %.loopexit119, label %.preheader118.preheader

.preheader118.preheader:                          ; preds = %155
  %172 = add i64 %168, -8
  %173 = lshr exact i64 %172, 3
  %174 = add nuw nsw i64 %173, 1
  %xtraiter = and i64 %174, 3
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0
  br i1 %lcmp.mod.not, label %.preheader118.prol.loopexit, label %.preheader118.prol

.preheader118.prol:                               ; preds = %.preheader118.preheader, %199
  %175 = phi ptr [ %177, %199 ], [ %164, %.preheader118.preheader ]
  %176 = phi <2 x i64> [ %200, %199 ], [ %167, %.preheader118.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %199 ], [ 0, %.preheader118.preheader ]
  %177 = getelementptr inbounds nuw i8, ptr %175, i64 8
  %178 = load i32, ptr %175, align 4, !range !1947, !noalias !44304, !noundef !1708
  %179 = icmp ne i32 %178, 2
  %180 = zext i1 %179 to i64
  %181 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %180, i64 0
  %182 = xor <2 x i64> %181, %176
  %183 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %182, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %184 = icmp eq i32 %178, 2
  br i1 %184, label %199, label %185

185:                                              ; preds = %.preheader118.prol
  %186 = getelementptr i8, ptr %175, i64 4
  %187 = load i32, ptr %186, align 4, !noalias !44304
  %188 = zext nneg i32 %178 to i64
  %189 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %188, i64 0
  %190 = xor <2 x i64> %189, %183
  %191 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %190, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %192 = or i32 %178, -2
  %193 = add nsw i32 %192, 1
  %194 = add i32 %193, %187
  %195 = zext i32 %194 to i64
  %196 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %195, i64 0
  %197 = xor <2 x i64> %196, %191
  %198 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %197, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  br label %199

199:                                              ; preds = %185, %.preheader118.prol
  %200 = phi <2 x i64> [ %198, %185 ], [ %183, %.preheader118.prol ]
  %prol.iter.next = add i64 %prol.iter, 1
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter
  br i1 %prol.iter.cmp.not, label %.preheader118.prol.loopexit, label %.preheader118.prol, !llvm.loop !44314

.preheader118.prol.loopexit:                      ; preds = %199, %.preheader118.preheader
  %.lcssa539.unr = phi <2 x i64> [ poison, %.preheader118.preheader ], [ %200, %199 ]
  %.unr = phi ptr [ %164, %.preheader118.preheader ], [ %177, %199 ]
  %.unr552 = phi <2 x i64> [ %167, %.preheader118.preheader ], [ %200, %199 ]
  %201 = icmp ult i64 %172, 24
  br i1 %201, label %.loopexit119, label %.preheader118

.preheader118:                                    ; preds = %.preheader118.prol.loopexit, %295
  %202 = phi ptr [ %273, %295 ], [ %.unr, %.preheader118.prol.loopexit ]
  %203 = phi <2 x i64> [ %296, %295 ], [ %.unr552, %.preheader118.prol.loopexit ]
  %204 = getelementptr inbounds nuw i8, ptr %202, i64 8
  %205 = load i32, ptr %202, align 4, !range !1947, !noalias !44304, !noundef !1708
  %206 = icmp ne i32 %205, 2
  %207 = zext i1 %206 to i64
  %208 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %207, i64 0
  %209 = xor <2 x i64> %208, %203
  %210 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %209, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %211 = icmp eq i32 %205, 2
  br i1 %211, label %.preheader118.1, label %212

212:                                              ; preds = %.preheader118
  %213 = getelementptr i8, ptr %202, i64 4
  %214 = load i32, ptr %213, align 4, !noalias !44304
  %215 = zext nneg i32 %205 to i64
  %216 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %215, i64 0
  %217 = xor <2 x i64> %216, %210
  %218 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %217, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %219 = or i32 %205, -2
  %220 = add nsw i32 %219, 1
  %221 = add i32 %220, %214
  %222 = zext i32 %221 to i64
  %223 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %222, i64 0
  %224 = xor <2 x i64> %223, %218
  %225 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %224, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  br label %.preheader118.1

.preheader118.1:                                  ; preds = %212, %.preheader118
  %226 = phi <2 x i64> [ %225, %212 ], [ %210, %.preheader118 ]
  %227 = getelementptr inbounds nuw i8, ptr %202, i64 16
  %228 = load i32, ptr %204, align 4, !range !1947, !noalias !44304, !noundef !1708
  %229 = icmp ne i32 %228, 2
  %230 = zext i1 %229 to i64
  %231 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %230, i64 0
  %232 = xor <2 x i64> %231, %226
  %233 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %232, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %234 = icmp eq i32 %228, 2
  br i1 %234, label %.preheader118.2, label %235

235:                                              ; preds = %.preheader118.1
  %236 = getelementptr i8, ptr %202, i64 12
  %237 = load i32, ptr %236, align 4, !noalias !44304
  %238 = zext nneg i32 %228 to i64
  %239 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %238, i64 0
  %240 = xor <2 x i64> %239, %233
  %241 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %240, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %242 = or i32 %228, -2
  %243 = add nsw i32 %242, 1
  %244 = add i32 %243, %237
  %245 = zext i32 %244 to i64
  %246 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %245, i64 0
  %247 = xor <2 x i64> %246, %241
  %248 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %247, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  br label %.preheader118.2

.preheader118.2:                                  ; preds = %235, %.preheader118.1
  %249 = phi <2 x i64> [ %248, %235 ], [ %233, %.preheader118.1 ]
  %250 = getelementptr inbounds nuw i8, ptr %202, i64 24
  %251 = load i32, ptr %227, align 4, !range !1947, !noalias !44304, !noundef !1708
  %252 = icmp ne i32 %251, 2
  %253 = zext i1 %252 to i64
  %254 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %253, i64 0
  %255 = xor <2 x i64> %254, %249
  %256 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %255, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %257 = icmp eq i32 %251, 2
  br i1 %257, label %.preheader118.3, label %258

258:                                              ; preds = %.preheader118.2
  %259 = getelementptr i8, ptr %202, i64 20
  %260 = load i32, ptr %259, align 4, !noalias !44304
  %261 = zext nneg i32 %251 to i64
  %262 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %261, i64 0
  %263 = xor <2 x i64> %262, %256
  %264 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %263, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %265 = or i32 %251, -2
  %266 = add nsw i32 %265, 1
  %267 = add i32 %266, %260
  %268 = zext i32 %267 to i64
  %269 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %268, i64 0
  %270 = xor <2 x i64> %269, %264
  %271 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %270, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  br label %.preheader118.3

.preheader118.3:                                  ; preds = %258, %.preheader118.2
  %272 = phi <2 x i64> [ %271, %258 ], [ %256, %.preheader118.2 ]
  %273 = getelementptr inbounds nuw i8, ptr %202, i64 32
  %274 = load i32, ptr %250, align 4, !range !1947, !noalias !44304, !noundef !1708
  %275 = icmp ne i32 %274, 2
  %276 = zext i1 %275 to i64
  %277 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %276, i64 0
  %278 = xor <2 x i64> %277, %272
  %279 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %278, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %280 = icmp eq i32 %274, 2
  br i1 %280, label %295, label %281

281:                                              ; preds = %.preheader118.3
  %282 = getelementptr i8, ptr %202, i64 28
  %283 = load i32, ptr %282, align 4, !noalias !44304
  %284 = zext nneg i32 %274 to i64
  %285 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %284, i64 0
  %286 = xor <2 x i64> %285, %279
  %287 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %286, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  %288 = or i32 %274, -2
  %289 = add nsw i32 %288, 1
  %290 = add i32 %289, %283
  %291 = zext i32 %290 to i64
  %292 = insertelement <2 x i64> <i64 poison, i64 0>, i64 %291, i64 0
  %293 = xor <2 x i64> %292, %287
  %294 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %293, <2 x i64> <i64 5582418988558022465, i64 7540007960554396517>)
  br label %295

295:                                              ; preds = %281, %.preheader118.3
  %296 = phi <2 x i64> [ %294, %281 ], [ %279, %.preheader118.3 ]
  %297 = icmp eq ptr %273, %169
  br i1 %297, label %.loopexit119, label %.preheader118

298:                                              ; preds = %436
  %299 = landingpad { ptr, i32 }
          cleanup
  store ptr %151, ptr %135, align 8, !noalias !44304
  store i64 %157, ptr %138, align 8, !noalias !44304
  %300 = icmp ugt i64 %152, 5
  br i1 %300, label %301, label %154

301:                                              ; preds = %298
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %160) ]
  %302 = shl i64 %152, 3
  %303 = add i64 %302, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %160, i64 noundef %303, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !44315
  br label %154

.loopexit119:                                     ; preds = %.preheader118.prol.loopexit, %295, %155
  %304 = phi <2 x i64> [ %167, %155 ], [ %.lcssa539.unr, %.preheader118.prol.loopexit ], [ %296, %295 ]
  %305 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %304, <2 x i64> <i64 2770419649501688277, i64 4912192508662405875>)
  %306 = call <2 x i64> @llvm.x86.aesni.aesenc(<2 x i64> %305, <2 x i64> <i64 4028393671950310427, i64 7405497087257227243>)
  %307 = extractelement <2 x i64> %306, i64 0
  call void @llvm.experimental.noalias.scope.decl(metadata !44320)
  call void @llvm.experimental.noalias.scope.decl(metadata !44323)
  %308 = lshr i64 %307, 57
  %309 = trunc nuw nsw i64 %308 to i8
  %310 = load i64, ptr %143, align 8, !alias.scope !44326, !noalias !44327, !noundef !1708
  %311 = load ptr, ptr %17, align 8, !alias.scope !44326, !noalias !44327, !nonnull !1708, !noundef !1708
  %312 = insertelement <16 x i8> poison, i8 %309, i64 0
  %313 = shufflevector <16 x i8> %312, <16 x i8> poison, <16 x i32> zeroinitializer
  br i1 %170, label %.preheader114, label %.preheader116

.preheader114:                                    ; preds = %.loopexit119, %325
  %314 = phi i64 [ %326, %325 ], [ 0, %.loopexit119 ]
  %315 = phi i64 [ %327, %325 ], [ %307, %.loopexit119 ]
  %316 = and i64 %315, %310
  %317 = getelementptr inbounds nuw i8, ptr %311, i64 %316
  %318 = load <16 x i8>, ptr %317, align 1, !noalias !44329
  %319 = icmp eq <16 x i8> %318, %313
  %320 = bitcast <16 x i1> %319 to i16
  %321 = icmp eq i16 %320, 0
  br i1 %321, label %.loopexit109, label %.preheader107

.loopexit109:                                     ; preds = %342, %.preheader114
  %322 = icmp eq <16 x i8> %318, splat (i8 -1)
  %323 = bitcast <16 x i1> %322 to i16
  %324 = icmp eq i16 %323, 0
  br i1 %324, label %325, label %.loopexit115, !prof !1803

325:                                              ; preds = %.loopexit109
  %326 = add i64 %314, 16
  %327 = add i64 %316, %326
  br label %.preheader114

.preheader107:                                    ; preds = %.preheader114, %342
  %328 = phi i16 [ %344, %342 ], [ %320, %.preheader114 ]
  %329 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %328, i1 true)
  %330 = zext nneg i16 %329 to i64
  %331 = add i64 %316, %330
  %332 = and i64 %331, %310
  %333 = sub nsw i64 0, %332
  %334 = getelementptr inbounds [48 x i8], ptr %311, i64 %333
  %335 = getelementptr inbounds i8, ptr %334, i64 -48
  %336 = load i64, ptr %335, align 8, !range !1940, !alias.scope !44332, !noalias !44337, !noundef !1708
  %337 = icmp ugt i64 %336, 5
  %338 = getelementptr inbounds i8, ptr %334, i64 -32
  %339 = load i64, ptr %338, align 8, !alias.scope !44332, !noalias !44337
  %340 = select i1 %337, i64 %339, i64 %336
  %341 = icmp eq i64 %340, 1
  br i1 %341, label %.loopexit106, label %342, !prof !3950

342:                                              ; preds = %.preheader107
  %343 = add i16 %328, -1
  %344 = and i16 %343, %328
  %345 = icmp eq i16 %344, 0
  br i1 %345, label %.loopexit109, label %.preheader107

.preheader116:                                    ; preds = %.loopexit119, %400
  %346 = phi i64 [ %401, %400 ], [ 0, %.loopexit119 ]
  %347 = phi i64 [ %402, %400 ], [ %307, %.loopexit119 ]
  %348 = and i64 %347, %310
  %349 = getelementptr inbounds nuw i8, ptr %311, i64 %348
  %350 = load <16 x i8>, ptr %349, align 1, !noalias !44329
  %351 = icmp eq <16 x i8> %350, %313
  %352 = bitcast <16 x i1> %351 to i16
  %353 = icmp eq i16 %352, 0
  br i1 %353, label %.loopexit111, label %.preheader110

.preheader110:                                    ; preds = %.preheader116, %.loopexit105
  %354 = phi i16 [ %398, %.loopexit105 ], [ %352, %.preheader116 ]
  %355 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %354, i1 true)
  %356 = zext nneg i16 %355 to i64
  %357 = add i64 %348, %356
  %358 = and i64 %357, %310
  %359 = sub nsw i64 0, %358
  %360 = getelementptr inbounds [48 x i8], ptr %311, i64 %359
  %361 = getelementptr inbounds i8, ptr %360, i64 -48
  %362 = load i64, ptr %361, align 8, !range !1940, !alias.scope !44332, !noalias !44337, !noundef !1708
  %363 = add i64 %362, -1
  %364 = icmp ugt i64 %363, 4
  %365 = getelementptr inbounds i8, ptr %360, i64 -40
  %366 = load ptr, ptr %365, align 8, !alias.scope !44332, !noalias !44337, !nonnull !1708
  %367 = getelementptr inbounds i8, ptr %360, i64 -32
  %368 = load i64, ptr %367, align 8, !alias.scope !44332, !noalias !44337
  %369 = add i64 %368, -1
  %370 = select i1 %364, i64 %369, i64 %363
  %371 = select i1 %364, ptr %366, ptr %365
  %372 = icmp eq i64 %370, %163
  br i1 %372, label %.preheader104, label %.loopexit105, !prof !3950

373:                                              ; preds = %392, %384
  %374 = add nuw i64 %376, 1
  %375 = icmp eq i64 %374, %163
  br i1 %375, label %.loopexit106, label %.preheader104

.preheader104:                                    ; preds = %.preheader110, %373
  %376 = phi i64 [ %374, %373 ], [ 0, %.preheader110 ]
  %377 = getelementptr inbounds nuw [8 x i8], ptr %371, i64 %376
  %378 = getelementptr inbounds nuw [8 x i8], ptr %164, i64 %376
  %379 = load i32, ptr %377, align 4, !range !1947, !noalias !44337, !noundef !1708
  %380 = load i32, ptr %378, align 4, !noalias !44304
  %381 = icmp eq i32 %379, 2
  %382 = icmp eq i32 %380, 2
  %383 = select i1 %381, i1 true, i1 %382
  br i1 %383, label %392, label %384

384:                                              ; preds = %.preheader104
  %385 = getelementptr i8, ptr %378, i64 4
  %386 = load i32, ptr %385, align 4, !noalias !44304
  %387 = getelementptr i8, ptr %377, i64 4
  %388 = load i32, ptr %387, align 4, !noalias !44337
  %389 = icmp eq i32 %379, %380
  %390 = icmp eq i32 %388, %386
  %391 = select i1 %389, i1 %390, i1 false
  br i1 %391, label %373, label %.loopexit105, !prof !3950

392:                                              ; preds = %.preheader104
  %393 = select i1 %381, i1 %382, i1 false
  br i1 %393, label %373, label %.loopexit105, !prof !3950

.loopexit111:                                     ; preds = %.loopexit105, %.preheader116
  %394 = icmp eq <16 x i8> %350, splat (i8 -1)
  %395 = bitcast <16 x i1> %394 to i16
  %396 = icmp eq i16 %395, 0
  br i1 %396, label %400, label %.loopexit115, !prof !1803

.loopexit105:                                     ; preds = %392, %384, %.preheader110
  %397 = add i16 %354, -1
  %398 = and i16 %397, %354
  %399 = icmp eq i16 %398, 0
  br i1 %399, label %.loopexit111, label %.preheader110

400:                                              ; preds = %.loopexit111
  %401 = add i64 %346, 16
  %402 = add i64 %348, %401
  br label %.preheader116

.loopexit106:                                     ; preds = %.preheader107, %373
  %403 = icmp ugt i64 %152, 5
  br i1 %403, label %404, label %1273

404:                                              ; preds = %.loopexit106
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %160) ]
  %405 = shl i64 %152, 3
  %406 = add i64 %405, -8
  %407 = load i64, ptr %146, align 8, !noalias !44341, !noundef !1708
  %408 = call i64 @llvm.umin.i64(i64 %406, i64 9223372036854775807)
  %409 = call i64 @llvm.ssub.sat.i64(i64 %407, i64 %408)
  store i64 %409, ptr %146, align 8, !noalias !44341
  %410 = load i64, ptr %147, align 8, !noalias !44341, !noundef !1708
  %411 = icmp slt i64 %409, %410
  br i1 %411, label %412, label %.preheader536

412:                                              ; preds = %404
  store i64 %409, ptr %147, align 8, !noalias !44341
  br label %.preheader536

.preheader536:                                    ; preds = %412, %404
  br label %413

413:                                              ; preds = %.preheader536, %416
  %414 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44341
  %415 = icmp slt i64 %414, 0
  br i1 %415, label %416, label %__rustc::__rust_dealloc (.exit67)

416:                                              ; preds = %413
  %417 = add nsw i64 %414, 1
  %418 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %414, i64 %417 acq_rel acquire, align 8, !noalias !44341
  %419 = extractvalue { i64, i1 } %418, 1
  br i1 %419, label %420, label %413

420:                                              ; preds = %416
  %421 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %408 monotonic, align 8, !noalias !44341
  %422 = call i64 @llvm.ssub.sat.i64(i64 %421, i64 %408)
  %423 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44341
  br label %424

424:                                              ; preds = %427, %420
  %425 = phi i64 [ %423, %420 ], [ %430, %427 ]
  %426 = icmp slt i64 %422, %425
  br i1 %426, label %427, label %431

427:                                              ; preds = %424
  %428 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %425, i64 %422 monotonic monotonic, align 8, !noalias !44341
  %429 = extractvalue { i64, i1 } %428, 1
  %430 = extractvalue { i64, i1 } %428, 0
  br i1 %429, label %431, label %424

431:                                              ; preds = %427, %424
  %432 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44341
  br label %__rustc::__rust_dealloc (.exit67)

__rustc::__rust_dealloc (.exit67): ; preds = %413, %431
  %433 = icmp ne i64 %406, 0
  call void @llvm.assume(i1 %433), !noalias !44341
  call void @free(ptr noundef nonnull %160) #88, !noalias !44341
  br label %1273

.loopexit115:                                     ; preds = %.loopexit111, %.loopexit109
  %434 = load i64, ptr %144, align 8, !alias.scope !44344, !noalias !44347, !noundef !1708
  %435 = icmp eq i64 %434, 0
  br i1 %435, label %436, label %1236, !prof !1803

436:                                              ; preds = %.loopexit115
; invoke <hashbrown::raw::RawTable<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize)>>::reserve_rehash::<hashbrown::map::make_hasher<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>::{closure#0}>
  %437 = invoke { i64, i64 } @<hashbrown::raw::RawTable<(purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize)>>::reserve_rehash::<hashbrown::map::make_hasher<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize, purrdf_hash::fixed::FixedState>::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %17, i64 noundef 1, ptr nonnull poison, i1 noundef zeroext true) #87
          to label %438 unwind label %298, !noalias !44347

438:                                              ; preds = %436
  %439 = load ptr, ptr %17, align 8, !alias.scope !44348, !noalias !44351
  %440 = load i64, ptr %143, align 8, !alias.scope !44348, !noalias !44351
  br label %1236

.loopexit120:                                     ; preds = %148, %129
  %441 = phi ptr [ %131, %129 ], [ %151, %148 ]
  %442 = ptrtoint ptr %134 to i64
  %443 = ptrtoint ptr %441 to i64
  %444 = sub nuw i64 %442, %443
  %445 = udiv exact i64 %444, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !44353)
  %446 = icmp eq ptr %134, %441
  br i1 %446, label %.loopexit103, label %.preheader102

.preheader102:                                    ; preds = %.loopexit120
  %447 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %448 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %449

449:                                              ; preds = %.preheader102, %487
  %450 = phi i64 [ %452, %487 ], [ 0, %.preheader102 ]
  %451 = getelementptr inbounds nuw [40 x i8], ptr %441, i64 %450
  %452 = add nuw nsw i64 %450, 1
  %453 = load i64, ptr %451, align 8, !range !1940, !alias.scope !44356, !noalias !44359, !noundef !1708
  %454 = icmp ugt i64 %453, 5
  br i1 %454, label %455, label %487

455:                                              ; preds = %449
  %456 = getelementptr i8, ptr %451, i64 8
  %457 = load ptr, ptr %456, align 8, !alias.scope !44353, !noalias !44359, !nonnull !1708, !noundef !1708
  %458 = shl i64 %453, 3
  %459 = add i64 %458, -8
  %460 = load i64, ptr %447, align 8, !noalias !44366, !noundef !1708
  %461 = call i64 @llvm.umin.i64(i64 %459, i64 9223372036854775807)
  %462 = call i64 @llvm.ssub.sat.i64(i64 %460, i64 %461)
  store i64 %462, ptr %447, align 8, !noalias !44366
  %463 = load i64, ptr %448, align 8, !noalias !44366, !noundef !1708
  %464 = icmp slt i64 %462, %463
  br i1 %464, label %465, label %.preheader534

465:                                              ; preds = %455
  store i64 %462, ptr %448, align 8, !noalias !44366
  br label %.preheader534

.preheader534:                                    ; preds = %465, %455
  br label %466

466:                                              ; preds = %.preheader534, %469
  %467 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44366
  %468 = icmp slt i64 %467, 0
  br i1 %468, label %469, label %__rustc::__rust_dealloc (.exit68)

469:                                              ; preds = %466
  %470 = add nsw i64 %467, 1
  %471 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %467, i64 %470 acq_rel acquire, align 8, !noalias !44366
  %472 = extractvalue { i64, i1 } %471, 1
  br i1 %472, label %473, label %466

473:                                              ; preds = %469
  %474 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %461 monotonic, align 8, !noalias !44366
  %475 = call i64 @llvm.ssub.sat.i64(i64 %474, i64 %461)
  %476 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44366
  br label %477

477:                                              ; preds = %480, %473
  %478 = phi i64 [ %476, %473 ], [ %483, %480 ]
  %479 = icmp slt i64 %475, %478
  br i1 %479, label %480, label %484

480:                                              ; preds = %477
  %481 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %478, i64 %475 monotonic monotonic, align 8, !noalias !44366
  %482 = extractvalue { i64, i1 } %481, 1
  %483 = extractvalue { i64, i1 } %481, 0
  br i1 %482, label %484, label %477

484:                                              ; preds = %480, %477
  %485 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44366
  br label %__rustc::__rust_dealloc (.exit68)

__rustc::__rust_dealloc (.exit68): ; preds = %466, %484
  %486 = icmp ne i64 %459, 0
  call void @llvm.assume(i1 %486), !noalias !44366
  call void @free(ptr noundef nonnull %457) #88, !noalias !44366
  br label %487

487:                                              ; preds = %__rustc::__rust_dealloc (.exit68), %449
  %488 = icmp eq i64 %452, %445
  br i1 %488, label %.loopexit103, label %449

.loopexit103:                                     ; preds = %1273, %487, %.loopexit120
  %489 = icmp eq i64 %132, 0
  br i1 %489, label %520, label %490

490:                                              ; preds = %.loopexit103
  %491 = mul nuw i64 %132, 40
  %492 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %493 = load i64, ptr %492, align 8, !noalias !44359, !noundef !1708
  %494 = call i64 @llvm.umin.i64(i64 %491, i64 9223372036854775807)
  %495 = call i64 @llvm.ssub.sat.i64(i64 %493, i64 %494)
  store i64 %495, ptr %492, align 8, !noalias !44359
  %496 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %497 = load i64, ptr %496, align 8, !noalias !44359, !noundef !1708
  %498 = icmp slt i64 %495, %497
  br i1 %498, label %499, label %.preheader533

499:                                              ; preds = %490
  store i64 %495, ptr %496, align 8, !noalias !44359
  br label %.preheader533

.preheader533:                                    ; preds = %499, %490
  br label %500

500:                                              ; preds = %.preheader533, %503
  %501 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44359
  %502 = icmp slt i64 %501, 0
  br i1 %502, label %503, label %__rustc::__rust_dealloc (.exit69)

503:                                              ; preds = %500
  %504 = add nsw i64 %501, 1
  %505 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %501, i64 %504 acq_rel acquire, align 8, !noalias !44359
  %506 = extractvalue { i64, i1 } %505, 1
  br i1 %506, label %507, label %500

507:                                              ; preds = %503
  %508 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %494 monotonic, align 8, !noalias !44359
  %509 = call i64 @llvm.ssub.sat.i64(i64 %508, i64 %494)
  %510 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44359
  br label %511

511:                                              ; preds = %514, %507
  %512 = phi i64 [ %510, %507 ], [ %517, %514 ]
  %513 = icmp slt i64 %509, %512
  br i1 %513, label %514, label %518

514:                                              ; preds = %511
  %515 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %512, i64 %509 monotonic monotonic, align 8, !noalias !44359
  %516 = extractvalue { i64, i1 } %515, 1
  %517 = extractvalue { i64, i1 } %515, 0
  br i1 %516, label %518, label %511

518:                                              ; preds = %514, %511
  %519 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44359
  br label %__rustc::__rust_dealloc (.exit69)

__rustc::__rust_dealloc (.exit69): ; preds = %500, %518
  call void @free(ptr noundef nonnull %131) #88, !noalias !44359
  br label %520

520:                                              ; preds = %__rustc::__rust_dealloc (.exit69), %.loopexit103
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !44304
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !44304
  %521 = load ptr, ptr %17, align 8, !noalias !44304, !nonnull !1708, !noundef !1708
  %522 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %523 = load i64, ptr %522, align 8, !noalias !44304
  %524 = getelementptr inbounds nuw i8, ptr %17, i64 24
  %525 = load i64, ptr %524, align 8, !noalias !44304
  %526 = load <16 x i8>, ptr %521, align 16, !noalias !44369
  %527 = icmp eq i64 %523, 0
  br i1 %527, label %537, label %528

528:                                              ; preds = %520
  %529 = mul i64 %523, 48
  %530 = add i64 %529, 48
  %531 = add i64 %523, 17
  %532 = add i64 %531, %530
  %533 = icmp uge i64 %532, %530
  call void @llvm.assume(i1 %533)
  %534 = icmp ult i64 %532, 9223372036854775793
  call void @llvm.assume(i1 %534)
  %535 = sub i64 -48, %529
  %536 = getelementptr inbounds i8, ptr %521, i64 %535
  br label %537

537:                                              ; preds = %528, %520
  %538 = phi i64 [ undef, %520 ], [ %532, %528 ]
  %539 = phi ptr [ undef, %520 ], [ %536, %528 ]
  %540 = phi i64 [ 0, %520 ], [ 16, %528 ]
  %541 = getelementptr i8, ptr %521, i64 %523
  %542 = getelementptr i8, ptr %541, i64 1
  %543 = ptrtoint ptr %542 to i64
  call void @llvm.experimental.noalias.scope.decl(metadata !44378)
  call void @llvm.experimental.noalias.scope.decl(metadata !44381)
  call void @llvm.lifetime.start.p0(ptr nonnull %9)
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !44384
  %544 = icmp eq i64 %525, 0
  br i1 %544, label %545, label %548

545:                                              ; preds = %537
  store i64 0, ptr %15, align 8, !alias.scope !44387, !noalias !44388
  %546 = getelementptr inbounds nuw i8, ptr %15, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %546, align 8, !alias.scope !44387, !noalias !44388
  %547 = getelementptr inbounds nuw i8, ptr %15, i64 16
  store i64 0, ptr %547, align 8, !alias.scope !44387, !noalias !44388
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !44384
  br label %.loopexit87

548:                                              ; preds = %537
  %549 = icmp sgt <16 x i8> %526, splat (i8 -1)
  %550 = bitcast <16 x i1> %549 to i16
  %551 = getelementptr inbounds nuw i8, ptr %521, i64 16
  %552 = icmp eq i16 %550, 0
  br i1 %552, label %.preheader100, label %.loopexit101

.preheader100:                                    ; preds = %548, %.preheader100
  %553 = phi ptr [ %558, %.preheader100 ], [ %551, %548 ]
  %554 = phi ptr [ %557, %.preheader100 ], [ %521, %548 ]
  %555 = load <16 x i8>, ptr %553, align 16, !noalias !44389
  %556 = icmp sgt <16 x i8> %555, splat (i8 -1)
  %557 = getelementptr inbounds i8, ptr %554, i64 -768
  %558 = getelementptr inbounds nuw i8, ptr %553, i64 16
  %559 = bitcast <16 x i1> %556 to i16
  %560 = icmp eq i16 %559, 0
  br i1 %560, label %.preheader100, label %.loopexit101

.loopexit101:                                     ; preds = %.preheader100, %548
  %561 = phi ptr [ %551, %548 ], [ %558, %.preheader100 ]
  %562 = phi ptr [ %521, %548 ], [ %557, %.preheader100 ]
  %563 = phi i16 [ %550, %548 ], [ %559, %.preheader100 ]
  %564 = add i16 %563, -1
  %565 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %563, i1 true)
  %566 = zext nneg i16 %565 to i64
  %567 = and i16 %564, %563
  %568 = sub nsw i64 0, %566
  %569 = getelementptr inbounds [48 x i8], ptr %562, i64 %568
  %570 = add i64 %525, -1
  %571 = getelementptr inbounds i8, ptr %569, i64 -48
  %572 = load i64, ptr %571, align 8, !noalias !44401
  %573 = icmp eq i64 %572, 0
  br i1 %573, label %574, label %706

574:                                              ; preds = %.loopexit101
  store i64 0, ptr %15, align 8, !alias.scope !44387, !noalias !44388
  %575 = getelementptr inbounds nuw i8, ptr %15, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %575, align 8, !alias.scope !44387, !noalias !44388
  %576 = getelementptr inbounds nuw i8, ptr %15, i64 16
  store i64 0, ptr %576, align 8, !alias.scope !44387, !noalias !44388
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !44384
  %577 = icmp eq i64 %570, 0
  br i1 %577, label %.loopexit87, label %.preheader86

.preheader86:                                     ; preds = %574
  %578 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %579 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %580

580:                                              ; preds = %.preheader86, %639
  %581 = phi ptr [ %594, %639 ], [ %561, %.preheader86 ]
  %582 = phi i64 [ %603, %639 ], [ %570, %.preheader86 ]
  %583 = phi ptr [ %595, %639 ], [ %562, %.preheader86 ]
  %584 = phi i16 [ %600, %639 ], [ %567, %.preheader86 ]
  %585 = icmp eq i16 %584, 0
  br i1 %585, label %.preheader84, label %.loopexit85

.preheader84:                                     ; preds = %580, %.preheader84
  %586 = phi ptr [ %591, %.preheader84 ], [ %581, %580 ]
  %587 = phi ptr [ %590, %.preheader84 ], [ %583, %580 ]
  %588 = load <16 x i8>, ptr %586, align 16, !noalias !44402
  %589 = icmp sgt <16 x i8> %588, splat (i8 -1)
  %590 = getelementptr inbounds i8, ptr %587, i64 -768
  %591 = getelementptr inbounds nuw i8, ptr %586, i64 16
  %592 = bitcast <16 x i1> %589 to i16
  %593 = icmp eq i16 %592, 0
  br i1 %593, label %.preheader84, label %.loopexit85

.loopexit85:                                      ; preds = %.preheader84, %580
  %594 = phi ptr [ %581, %580 ], [ %591, %.preheader84 ]
  %595 = phi ptr [ %583, %580 ], [ %590, %.preheader84 ]
  %596 = phi i16 [ %584, %580 ], [ %592, %.preheader84 ]
  %597 = add i16 %596, -1
  %598 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %596, i1 true)
  %599 = zext nneg i16 %598 to i64
  %600 = and i16 %597, %596
  %601 = sub nsw i64 0, %599
  %602 = getelementptr inbounds [48 x i8], ptr %595, i64 %601
  %603 = add i64 %582, -1
  %604 = getelementptr inbounds i8, ptr %602, i64 -48
  %605 = load i64, ptr %604, align 8, !range !1940, !alias.scope !44417, !noalias !44420, !noundef !1708
  %606 = icmp ugt i64 %605, 5
  br i1 %606, label %607, label %639

607:                                              ; preds = %.loopexit85
  %608 = getelementptr i8, ptr %602, i64 -40
  %609 = load ptr, ptr %608, align 8, !noalias !44420, !nonnull !1708, !noundef !1708
  %610 = shl i64 %605, 3
  %611 = add i64 %610, -8
  %612 = load i64, ptr %578, align 8, !noalias !44421, !noundef !1708
  %613 = call i64 @llvm.umin.i64(i64 %611, i64 9223372036854775807)
  %614 = call i64 @llvm.ssub.sat.i64(i64 %612, i64 %613)
  store i64 %614, ptr %578, align 8, !noalias !44421
  %615 = load i64, ptr %579, align 8, !noalias !44421, !noundef !1708
  %616 = icmp slt i64 %614, %615
  br i1 %616, label %617, label %.preheader487

617:                                              ; preds = %607
  store i64 %614, ptr %579, align 8, !noalias !44421
  br label %.preheader487

.preheader487:                                    ; preds = %617, %607
  br label %618

618:                                              ; preds = %.preheader487, %621
  %619 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44421
  %620 = icmp slt i64 %619, 0
  br i1 %620, label %621, label %__rustc::__rust_dealloc (.exit70)

621:                                              ; preds = %618
  %622 = add nsw i64 %619, 1
  %623 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %619, i64 %622 acq_rel acquire, align 8, !noalias !44421
  %624 = extractvalue { i64, i1 } %623, 1
  br i1 %624, label %625, label %618

625:                                              ; preds = %621
  %626 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %613 monotonic, align 8, !noalias !44421
  %627 = call i64 @llvm.ssub.sat.i64(i64 %626, i64 %613)
  %628 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44421
  br label %629

629:                                              ; preds = %632, %625
  %630 = phi i64 [ %628, %625 ], [ %635, %632 ]
  %631 = icmp slt i64 %627, %630
  br i1 %631, label %632, label %636

632:                                              ; preds = %629
  %633 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %630, i64 %627 monotonic monotonic, align 8, !noalias !44421
  %634 = extractvalue { i64, i1 } %633, 1
  %635 = extractvalue { i64, i1 } %633, 0
  br i1 %634, label %636, label %629

636:                                              ; preds = %632, %629
  %637 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44421
  br label %__rustc::__rust_dealloc (.exit70)

__rustc::__rust_dealloc (.exit70): ; preds = %618, %636
  %638 = icmp ne i64 %611, 0
  call void @llvm.assume(i1 %638), !noalias !44421
  call void @free(ptr noundef nonnull %609) #88, !noalias !44421
  br label %639

639:                                              ; preds = %__rustc::__rust_dealloc (.exit70), %.loopexit85
  %640 = icmp eq i64 %603, 0
  br i1 %640, label %.loopexit87, label %580

.loopexit87:                                      ; preds = %639, %574, %545
  %641 = icmp eq i64 %538, 0
  %642 = or i1 %527, %641
  br i1 %642, label %1025, label %643

643:                                              ; preds = %.loopexit87
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %539) ]
  %644 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %645 = load i64, ptr %644, align 8, !noalias !44424, !noundef !1708
  %646 = call i64 @llvm.ssub.sat.i64(i64 %645, i64 %538)
  store i64 %646, ptr %644, align 8, !noalias !44424
  %647 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %648 = load i64, ptr %647, align 8, !noalias !44424, !noundef !1708
  %649 = icmp slt i64 %646, %648
  br i1 %649, label %650, label %.preheader486

650:                                              ; preds = %643
  store i64 %646, ptr %647, align 8, !noalias !44424
  br label %.preheader486

.preheader486:                                    ; preds = %650, %643
  br label %651

651:                                              ; preds = %.preheader486, %654
  %652 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44424
  %653 = icmp slt i64 %652, 0
  br i1 %653, label %654, label %__rustc::__rust_dealloc (.exit71)

654:                                              ; preds = %651
  %655 = add nsw i64 %652, 1
  %656 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %652, i64 %655 acq_rel acquire, align 8, !noalias !44424
  %657 = extractvalue { i64, i1 } %656, 1
  br i1 %657, label %658, label %651

658:                                              ; preds = %654
  %659 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %538 monotonic, align 8, !noalias !44424
  %660 = call i64 @llvm.ssub.sat.i64(i64 %659, i64 %538)
  %661 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44424
  br label %662

662:                                              ; preds = %665, %658
  %663 = phi i64 [ %661, %658 ], [ %668, %665 ]
  %664 = icmp slt i64 %660, %663
  br i1 %664, label %665, label %669

665:                                              ; preds = %662
  %666 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %663, i64 %660 monotonic monotonic, align 8, !noalias !44424
  %667 = extractvalue { i64, i1 } %666, 1
  %668 = extractvalue { i64, i1 } %666, 0
  br i1 %667, label %669, label %662

669:                                              ; preds = %665, %662
  %670 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44424
  br label %__rustc::__rust_dealloc (.exit71)

__rustc::__rust_dealloc (.exit71): ; preds = %651, %669
  call void @free(ptr noundef nonnull %539) #88, !noalias !44424
  br label %1025

671:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  %672 = landingpad { ptr, i32 }
          cleanup
  %673 = icmp ugt i64 %572, 5
  br i1 %673, label %674, label %930

674:                                              ; preds = %671
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %710) ]
  %675 = shl i64 %572, 3
  %676 = add i64 %675, -8
  %677 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %678 = load i64, ptr %677, align 8, !noalias !44425, !noundef !1708
  %679 = call i64 @llvm.umin.i64(i64 %676, i64 9223372036854775807)
  %680 = call i64 @llvm.ssub.sat.i64(i64 %678, i64 %679)
  store i64 %680, ptr %677, align 8, !noalias !44425
  %681 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %682 = load i64, ptr %681, align 8, !noalias !44425, !noundef !1708
  %683 = icmp slt i64 %680, %682
  br i1 %683, label %684, label %.preheader496

684:                                              ; preds = %674
  store i64 %680, ptr %681, align 8, !noalias !44425
  br label %.preheader496

.preheader496:                                    ; preds = %684, %674
  br label %685

685:                                              ; preds = %.preheader496, %688
  %686 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44425
  %687 = icmp slt i64 %686, 0
  br i1 %687, label %688, label %__rustc::__rust_dealloc (.exit72)

688:                                              ; preds = %685
  %689 = add nsw i64 %686, 1
  %690 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %686, i64 %689 acq_rel acquire, align 8, !noalias !44425
  %691 = extractvalue { i64, i1 } %690, 1
  br i1 %691, label %692, label %685

692:                                              ; preds = %688
  %693 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %679 monotonic, align 8, !noalias !44425
  %694 = call i64 @llvm.ssub.sat.i64(i64 %693, i64 %679)
  %695 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44425
  br label %696

696:                                              ; preds = %699, %692
  %697 = phi i64 [ %695, %692 ], [ %702, %699 ]
  %698 = icmp slt i64 %694, %697
  br i1 %698, label %699, label %703

699:                                              ; preds = %696
  %700 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %697, i64 %694 monotonic monotonic, align 8, !noalias !44425
  %701 = extractvalue { i64, i1 } %700, 1
  %702 = extractvalue { i64, i1 } %700, 0
  br i1 %701, label %703, label %696

703:                                              ; preds = %699, %696
  %704 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44425
  br label %__rustc::__rust_dealloc (.exit72)

__rustc::__rust_dealloc (.exit72): ; preds = %685, %703
  %705 = icmp ne i64 %676, 0
  call void @llvm.assume(i1 %705), !noalias !44425
  call void @free(ptr noundef nonnull %710) #88, !noalias !44425
  br label %930

706:                                              ; preds = %.loopexit101
  %707 = getelementptr inbounds i8, ptr %569, i64 -40
  %708 = getelementptr inbounds i8, ptr %569, i64 -8
  %709 = load i64, ptr %708, align 8, !noalias !44401
  %710 = load ptr, ptr %707, align 8, !noalias !44428
  %711 = getelementptr inbounds i8, ptr %569, i64 -32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %9, ptr noundef nonnull align 8 dereferenceable(24) %711, i64 24, i1 false), !noalias !44384
  %712 = call i64 @llvm.umax.i64(i64 %525, i64 4)
  %713 = mul i64 %712, 48
  %714 = icmp ugt i64 %525, 192153584101141162
  br i1 %714, label %__rustc::__rust_alloc (.exit.thread), label %715, !prof !5895

715:                                              ; preds = %706
  %716 = icmp eq i64 %713, 0
  br i1 %716, label %__rustc::__rust_alloc (.exit), label %717

717:                                              ; preds = %715
  %718 = call noundef ptr @malloc(i64 noundef range(i64 1, 0) %713) #88, !noalias !44429
  %719 = icmp eq ptr %718, null
  br i1 %719, label %__rustc::__rust_alloc (.exit.thread), label %720

720:                                              ; preds = %717
  %721 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_ALLOCATIONS::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %722 = load i64, ptr %721, align 8, !noalias !44429, !noundef !1708
  %723 = call i64 @llvm.uadd.sat.i64(i64 %722, i64 1)
  store i64 %723, ptr %721, align 8, !noalias !44429
  %724 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_REQUESTED_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %725 = load i64, ptr %724, align 8, !noalias !44429, !noundef !1708
  %726 = call i64 @llvm.uadd.sat.i64(i64 %725, i64 %713)
  store i64 %726, ptr %724, align 8, !noalias !44429
  %727 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %728 = load i64, ptr %727, align 8, !noalias !44429, !noundef !1708
  %729 = call i64 @llvm.umin.i64(i64 %713, i64 9223372036854775807)
  %730 = call i64 @llvm.sadd.sat.i64(i64 %728, i64 %729)
  store i64 %730, ptr %727, align 8, !noalias !44429
  %731 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_PEAK_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %732 = load i64, ptr %731, align 8, !noalias !44429, !noundef !1708
  %733 = icmp sgt i64 %730, %732
  br i1 %733, label %734, label %.preheader529

734:                                              ; preds = %720
  store i64 %730, ptr %731, align 8, !noalias !44429
  br label %.preheader529

.preheader529:                                    ; preds = %734, %720
  br label %735

735:                                              ; preds = %.preheader529, %738
  %736 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44429
  %737 = icmp slt i64 %736, 0
  br i1 %737, label %738, label %__rustc::__rust_alloc (.exit)

738:                                              ; preds = %735
  %739 = add nsw i64 %736, 1
  %740 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %736, i64 %739 acq_rel acquire, align 8, !noalias !44429
  %741 = extractvalue { i64, i1 } %740, 1
  br i1 %741, label %742, label %735

742:                                              ; preds = %738
  %743 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_ALLOCATIONS, i64 1 monotonic, align 8, !noalias !44429
  %744 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_REQUESTED_BYTES, i64 %713 monotonic, align 8, !noalias !44429
  %745 = atomicrmw add ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %729 monotonic, align 8, !noalias !44429
  %746 = call i64 @llvm.sadd.sat.i64(i64 %745, i64 %729)
  %747 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES monotonic, align 8, !noalias !44429
  br label %748

748:                                              ; preds = %751, %742
  %749 = phi i64 [ %747, %742 ], [ %754, %751 ]
  %750 = icmp sgt i64 %746, %749
  br i1 %750, label %751, label %755

751:                                              ; preds = %748
  %752 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_PEAK_BYTES, i64 %749, i64 %746 monotonic monotonic, align 8, !noalias !44429
  %753 = extractvalue { i64, i1 } %752, 1
  %754 = extractvalue { i64, i1 } %752, 0
  br i1 %753, label %755, label %748

755:                                              ; preds = %751, %748
  %756 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44429
  br label %__rustc::__rust_alloc (.exit)

__rustc::__rust_alloc (.exit.thread): ; preds = %717, %706
  %757 = phi i64 [ 8, %717 ], [ 0, %706 ]
; invoke alloc::raw_vec::handle_error
  invoke void @alloc::raw_vec::handle_error(i64 noundef %757, i64 %713) #90
          to label %758 unwind label %671, !noalias !44384

758:                                              ; preds = %__rustc::__rust_alloc (.exit.thread)
  unreachable

__rustc::__rust_alloc (.exit):  ; preds = %735, %755, %715
  %759 = phi i64 [ 0, %715 ], [ %712, %755 ], [ %712, %735 ]
  %760 = phi ptr [ inttoptr (i64 8 to ptr), %715 ], [ %718, %755 ], [ %718, %735 ]
  %761 = icmp ule i64 %712, %759
  call void @llvm.assume(i1 %761)
  store i64 %709, ptr %760, align 8, !noalias !44384
  %762 = getelementptr inbounds nuw i8, ptr %760, i64 8
  store i64 %572, ptr %762, align 8, !noalias !44384
  %763 = getelementptr inbounds nuw i8, ptr %760, i64 16
  store ptr %710, ptr %763, align 8, !noalias !44384
  %764 = getelementptr inbounds nuw i8, ptr %760, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %764, ptr noundef nonnull align 8 dereferenceable(24) %9, i64 24, i1 false), !noalias !44384
  store i64 %759, ptr %10, align 8, !noalias !44384
  %765 = getelementptr inbounds nuw i8, ptr %10, i64 8
  store ptr %760, ptr %765, align 8, !noalias !44384
  %766 = getelementptr inbounds nuw i8, ptr %10, i64 16
  store i64 1, ptr %766, align 8, !noalias !44384
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !44384
  store i64 %540, ptr %8, align 8, !noalias !44432
  %767 = getelementptr inbounds nuw i8, ptr %8, i64 8
  store i64 %538, ptr %767, align 8, !noalias !44432
  %768 = getelementptr inbounds nuw i8, ptr %8, i64 16
  store ptr %539, ptr %768, align 8, !noalias !44432
  %769 = getelementptr inbounds nuw i8, ptr %8, i64 24
  store ptr %562, ptr %769, align 8, !noalias !44432
  %770 = getelementptr inbounds nuw i8, ptr %8, i64 32
  store ptr %561, ptr %770, align 8, !noalias !44432
  %771 = getelementptr inbounds nuw i8, ptr %8, i64 40
  store i64 %543, ptr %771, align 8, !noalias !44432
  %772 = getelementptr inbounds nuw i8, ptr %8, i64 48
  store i16 %567, ptr %772, align 8, !noalias !44432
  %773 = getelementptr inbounds nuw i8, ptr %8, i64 56
  store i64 %570, ptr %773, align 8, !noalias !44432
  call void @llvm.experimental.noalias.scope.decl(metadata !44433)
  call void @llvm.experimental.noalias.scope.decl(metadata !44436)
  call void @llvm.experimental.noalias.scope.decl(metadata !44438)
  call void @llvm.experimental.noalias.scope.decl(metadata !44441)
  call void @llvm.lifetime.start.p0(ptr nonnull %7)
  %774 = icmp eq i64 %570, 0
  br i1 %774, label %.loopexit95, label %.preheader98

.preheader98:                                     ; preds = %__rustc::__rust_alloc (.exit), %912
  %775 = phi ptr [ %913, %912 ], [ %760, %__rustc::__rust_alloc (.exit) ]
  %776 = phi i64 [ %918, %912 ], [ 1, %__rustc::__rust_alloc (.exit) ]
  %777 = phi ptr [ %792, %912 ], [ %562, %__rustc::__rust_alloc (.exit) ]
  %778 = phi ptr [ %793, %912 ], [ %561, %__rustc::__rust_alloc (.exit) ]
  %779 = phi ptr [ %794, %912 ], [ %561, %__rustc::__rust_alloc (.exit) ]
  %780 = phi ptr [ %795, %912 ], [ %562, %__rustc::__rust_alloc (.exit) ]
  %781 = phi i16 [ %800, %912 ], [ %567, %__rustc::__rust_alloc (.exit) ]
  %782 = phi i64 [ %803, %912 ], [ %570, %__rustc::__rust_alloc (.exit) ]
  call void @llvm.experimental.noalias.scope.decl(metadata !44443)
  call void @llvm.experimental.noalias.scope.decl(metadata !44446)
  call void @llvm.experimental.noalias.scope.decl(metadata !44449)
  call void @llvm.experimental.noalias.scope.decl(metadata !44452)
  %783 = icmp eq i16 %781, 0
  br i1 %783, label %.preheader96, label %.loopexit97

.preheader96:                                     ; preds = %.preheader98, %.preheader96
  %784 = phi ptr [ %789, %.preheader96 ], [ %779, %.preheader98 ]
  %785 = phi ptr [ %788, %.preheader96 ], [ %780, %.preheader98 ]
  %786 = load <16 x i8>, ptr %784, align 16, !noalias !44455
  %787 = icmp sgt <16 x i8> %786, splat (i8 -1)
  %788 = getelementptr inbounds i8, ptr %785, i64 -768
  %789 = getelementptr inbounds nuw i8, ptr %784, i64 16
  %790 = bitcast <16 x i1> %787 to i16
  %791 = icmp eq i16 %790, 0
  br i1 %791, label %.preheader96, label %.loopexit97

.loopexit97:                                      ; preds = %.preheader96, %.preheader98
  %792 = phi ptr [ %777, %.preheader98 ], [ %788, %.preheader96 ]
  %793 = phi ptr [ %778, %.preheader98 ], [ %789, %.preheader96 ]
  %794 = phi ptr [ %779, %.preheader98 ], [ %789, %.preheader96 ]
  %795 = phi ptr [ %780, %.preheader98 ], [ %788, %.preheader96 ]
  %796 = phi i16 [ %781, %.preheader98 ], [ %790, %.preheader96 ]
  %797 = add i16 %796, -1
  %798 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %796, i1 true)
  %799 = zext nneg i16 %798 to i64
  %800 = and i16 %797, %796
  %801 = sub nsw i64 0, %799
  %802 = getelementptr inbounds [48 x i8], ptr %795, i64 %801
  %803 = add i64 %782, -1
  %804 = getelementptr inbounds i8, ptr %802, i64 -48
  %805 = load i64, ptr %804, align 8, !noalias !44459
  %806 = icmp eq i64 %805, 0
  br i1 %806, label %817, label %808

807:                                              ; preds = %923, %920
; call core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#0}>>
  call fastcc void @core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, usize>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#0}>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(64) %8) #89, !noalias !44460
; call core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %10) #89, !noalias !44384
  br label %1296

808:                                              ; preds = %.loopexit97
  %809 = getelementptr inbounds i8, ptr %802, i64 -40
  %810 = getelementptr inbounds i8, ptr %802, i64 -8
  %811 = load i64, ptr %810, align 8, !noalias !44459
  %812 = load ptr, ptr %809, align 8, !noalias !44461
  %813 = getelementptr inbounds i8, ptr %802, i64 -32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %7, ptr noundef nonnull align 8 dereferenceable(24) %813, i64 24, i1 false), !noalias !44462
  %814 = icmp samesign ult i64 %776, 192153584101141163
  call void @llvm.assume(i1 %814)
  %815 = load i64, ptr %10, align 8, !range !1817, !alias.scope !44463, !noalias !44464, !noundef !1708
  %816 = icmp eq i64 %776, %815
  br i1 %816, label %926, label %912

817:                                              ; preds = %.loopexit97
  store ptr %793, ptr %770, align 8, !noalias !44384
  store ptr %792, ptr %769, align 8, !noalias !44384
  %818 = icmp eq i64 %803, 0
  br i1 %818, label %.loopexit95, label %.preheader94

.preheader94:                                     ; preds = %817
  %819 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %820 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %821

821:                                              ; preds = %.preheader94, %880
  %822 = phi ptr [ %835, %880 ], [ %794, %.preheader94 ]
  %823 = phi i64 [ %844, %880 ], [ %803, %.preheader94 ]
  %824 = phi ptr [ %836, %880 ], [ %795, %.preheader94 ]
  %825 = phi i16 [ %841, %880 ], [ %800, %.preheader94 ]
  %826 = icmp eq i16 %825, 0
  br i1 %826, label %.preheader92, label %.loopexit93

.preheader92:                                     ; preds = %821, %.preheader92
  %827 = phi ptr [ %832, %.preheader92 ], [ %822, %821 ]
  %828 = phi ptr [ %831, %.preheader92 ], [ %824, %821 ]
  %829 = load <16 x i8>, ptr %827, align 16, !noalias !44465
  %830 = icmp sgt <16 x i8> %829, splat (i8 -1)
  %831 = getelementptr inbounds i8, ptr %828, i64 -768
  %832 = getelementptr inbounds nuw i8, ptr %827, i64 16
  %833 = bitcast <16 x i1> %830 to i16
  %834 = icmp eq i16 %833, 0
  br i1 %834, label %.preheader92, label %.loopexit93

.loopexit93:                                      ; preds = %.preheader92, %821
  %835 = phi ptr [ %822, %821 ], [ %832, %.preheader92 ]
  %836 = phi ptr [ %824, %821 ], [ %831, %.preheader92 ]
  %837 = phi i16 [ %825, %821 ], [ %833, %.preheader92 ]
  %838 = add i16 %837, -1
  %839 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %837, i1 true)
  %840 = zext nneg i16 %839 to i64
  %841 = and i16 %838, %837
  %842 = sub nsw i64 0, %840
  %843 = getelementptr inbounds [48 x i8], ptr %836, i64 %842
  %844 = add i64 %823, -1
  %845 = getelementptr inbounds i8, ptr %843, i64 -48
  %846 = load i64, ptr %845, align 8, !range !1940, !alias.scope !44480, !noalias !44483, !noundef !1708
  %847 = icmp ugt i64 %846, 5
  br i1 %847, label %848, label %880

848:                                              ; preds = %.loopexit93
  %849 = getelementptr i8, ptr %843, i64 -40
  %850 = load ptr, ptr %849, align 8, !noalias !44483, !nonnull !1708, !noundef !1708
  %851 = shl i64 %846, 3
  %852 = add i64 %851, -8
  %853 = load i64, ptr %819, align 8, !noalias !44484, !noundef !1708
  %854 = call i64 @llvm.umin.i64(i64 %852, i64 9223372036854775807)
  %855 = call i64 @llvm.ssub.sat.i64(i64 %853, i64 %854)
  store i64 %855, ptr %819, align 8, !noalias !44484
  %856 = load i64, ptr %820, align 8, !noalias !44484, !noundef !1708
  %857 = icmp slt i64 %855, %856
  br i1 %857, label %858, label %.preheader498

858:                                              ; preds = %848
  store i64 %855, ptr %820, align 8, !noalias !44484
  br label %.preheader498

.preheader498:                                    ; preds = %858, %848
  br label %859

859:                                              ; preds = %.preheader498, %862
  %860 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44484
  %861 = icmp slt i64 %860, 0
  br i1 %861, label %862, label %__rustc::__rust_dealloc (.exit73)

862:                                              ; preds = %859
  %863 = add nsw i64 %860, 1
  %864 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %860, i64 %863 acq_rel acquire, align 8, !noalias !44484
  %865 = extractvalue { i64, i1 } %864, 1
  br i1 %865, label %866, label %859

866:                                              ; preds = %862
  %867 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %854 monotonic, align 8, !noalias !44484
  %868 = call i64 @llvm.ssub.sat.i64(i64 %867, i64 %854)
  %869 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44484
  br label %870

870:                                              ; preds = %873, %866
  %871 = phi i64 [ %869, %866 ], [ %876, %873 ]
  %872 = icmp slt i64 %868, %871
  br i1 %872, label %873, label %877

873:                                              ; preds = %870
  %874 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %871, i64 %868 monotonic monotonic, align 8, !noalias !44484
  %875 = extractvalue { i64, i1 } %874, 1
  %876 = extractvalue { i64, i1 } %874, 0
  br i1 %875, label %877, label %870

877:                                              ; preds = %873, %870
  %878 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44484
  br label %__rustc::__rust_dealloc (.exit73)

__rustc::__rust_dealloc (.exit73): ; preds = %859, %877
  %879 = icmp ne i64 %852, 0
  call void @llvm.assume(i1 %879), !noalias !44484
  call void @free(ptr noundef nonnull %850) #88, !noalias !44484
  br label %880

880:                                              ; preds = %__rustc::__rust_dealloc (.exit73), %.loopexit93
  %881 = icmp eq i64 %844, 0
  br i1 %881, label %.loopexit95, label %821

.loopexit95:                                      ; preds = %912, %880, %817, %__rustc::__rust_alloc (.exit)
  %882 = icmp eq i64 %538, 0
  %883 = or i1 %527, %882
  br i1 %883, label %1026, label %884

884:                                              ; preds = %.loopexit95
  %885 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %886 = load i64, ptr %885, align 8, !noalias !44487, !noundef !1708
  %887 = call i64 @llvm.ssub.sat.i64(i64 %886, i64 %538)
  store i64 %887, ptr %885, align 8, !noalias !44487
  %888 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %889 = load i64, ptr %888, align 8, !noalias !44487, !noundef !1708
  %890 = icmp slt i64 %887, %889
  br i1 %890, label %891, label %.preheader497

891:                                              ; preds = %884
  store i64 %887, ptr %888, align 8, !noalias !44487
  br label %.preheader497

.preheader497:                                    ; preds = %891, %884
  br label %892

892:                                              ; preds = %.preheader497, %895
  %893 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44487
  %894 = icmp slt i64 %893, 0
  br i1 %894, label %895, label %__rustc::__rust_dealloc (.exit74)

895:                                              ; preds = %892
  %896 = add nsw i64 %893, 1
  %897 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %893, i64 %896 acq_rel acquire, align 8, !noalias !44487
  %898 = extractvalue { i64, i1 } %897, 1
  br i1 %898, label %899, label %892

899:                                              ; preds = %895
  %900 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %538 monotonic, align 8, !noalias !44487
  %901 = call i64 @llvm.ssub.sat.i64(i64 %900, i64 %538)
  %902 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44487
  br label %903

903:                                              ; preds = %906, %899
  %904 = phi i64 [ %902, %899 ], [ %909, %906 ]
  %905 = icmp slt i64 %901, %904
  br i1 %905, label %906, label %910

906:                                              ; preds = %903
  %907 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %904, i64 %901 monotonic monotonic, align 8, !noalias !44487
  %908 = extractvalue { i64, i1 } %907, 1
  %909 = extractvalue { i64, i1 } %907, 0
  br i1 %908, label %910, label %903

910:                                              ; preds = %906, %903
  %911 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44487
  br label %__rustc::__rust_dealloc (.exit74)

__rustc::__rust_dealloc (.exit74): ; preds = %892, %910
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %539) ], !noalias !44487
  call void @free(ptr noundef nonnull %539) #88, !noalias !44487
  br label %1026

912:                                              ; preds = %928, %808
  %913 = phi ptr [ %929, %928 ], [ %775, %808 ]
  %914 = getelementptr inbounds nuw [48 x i8], ptr %913, i64 %776
  store i64 %811, ptr %914, align 8, !noalias !44462
  %915 = getelementptr inbounds nuw i8, ptr %914, i64 8
  store i64 %805, ptr %915, align 8, !noalias !44462
  %916 = getelementptr inbounds nuw i8, ptr %914, i64 16
  store ptr %812, ptr %916, align 8, !noalias !44462
  %917 = getelementptr inbounds nuw i8, ptr %914, i64 24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %917, ptr noundef nonnull align 8 dereferenceable(24) %7, i64 24, i1 false), !noalias !44462
  %918 = add nuw nsw i64 %776, 1
  store i64 %918, ptr %766, align 8, !alias.scope !44463, !noalias !44464
  %919 = icmp eq i64 %803, 0
  br i1 %919, label %.loopexit95, label %.preheader98

920:                                              ; preds = %926
  %921 = landingpad { ptr, i32 }
          cleanup
  store ptr %793, ptr %770, align 8, !noalias !44384
  store ptr %792, ptr %769, align 8, !noalias !44384
  store i16 %800, ptr %772, align 8, !alias.scope !44488, !noalias !44489
  store i64 %803, ptr %773, align 8, !alias.scope !44490, !noalias !44489
  %922 = icmp ugt i64 %805, 5
  br i1 %922, label %923, label %807

923:                                              ; preds = %920
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %812) ]
  %924 = shl i64 %805, 3
  %925 = add i64 %924, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %812, i64 noundef %925, i64 noundef range(i64 1, -9223372036854775807) 4) #88, !noalias !44491
  br label %807

926:                                              ; preds = %808
  %927 = call i64 @llvm.uadd.sat.i64(i64 %803, i64 1)
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.13412714042204560522)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %10, i64 noundef %776, i64 noundef range(i64 1, 0) %927, i64 noundef 8, i64 noundef 48)
          to label %928 unwind label %920, !noalias !44464

928:                                              ; preds = %926
  %929 = load ptr, ptr %765, align 8, !alias.scope !44463, !noalias !44464
  br label %912

930:                                              ; preds = %__rustc::__rust_dealloc (.exit72), %671
  %931 = icmp eq i64 %570, 0
  br i1 %931, label %.loopexit91, label %.preheader90

.preheader90:                                     ; preds = %930
  %932 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %933 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %934

934:                                              ; preds = %.preheader90, %993
  %935 = phi ptr [ %948, %993 ], [ %561, %.preheader90 ]
  %936 = phi i64 [ %957, %993 ], [ %570, %.preheader90 ]
  %937 = phi ptr [ %949, %993 ], [ %562, %.preheader90 ]
  %938 = phi i16 [ %954, %993 ], [ %567, %.preheader90 ]
  %939 = icmp eq i16 %938, 0
  br i1 %939, label %.preheader88, label %.loopexit89

.preheader88:                                     ; preds = %934, %.preheader88
  %940 = phi ptr [ %945, %.preheader88 ], [ %935, %934 ]
  %941 = phi ptr [ %944, %.preheader88 ], [ %937, %934 ]
  %942 = load <16 x i8>, ptr %940, align 16, !noalias !44494
  %943 = icmp sgt <16 x i8> %942, splat (i8 -1)
  %944 = getelementptr inbounds i8, ptr %941, i64 -768
  %945 = getelementptr inbounds nuw i8, ptr %940, i64 16
  %946 = bitcast <16 x i1> %943 to i16
  %947 = icmp eq i16 %946, 0
  br i1 %947, label %.preheader88, label %.loopexit89

.loopexit89:                                      ; preds = %.preheader88, %934
  %948 = phi ptr [ %935, %934 ], [ %945, %.preheader88 ]
  %949 = phi ptr [ %937, %934 ], [ %944, %.preheader88 ]
  %950 = phi i16 [ %938, %934 ], [ %946, %.preheader88 ]
  %951 = add i16 %950, -1
  %952 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %950, i1 true)
  %953 = zext nneg i16 %952 to i64
  %954 = and i16 %951, %950
  %955 = sub nsw i64 0, %953
  %956 = getelementptr inbounds [48 x i8], ptr %949, i64 %955
  %957 = add i64 %936, -1
  %958 = getelementptr inbounds i8, ptr %956, i64 -48
  %959 = load i64, ptr %958, align 8, !range !1940, !alias.scope !44509, !noalias !44512, !noundef !1708
  %960 = icmp ugt i64 %959, 5
  br i1 %960, label %961, label %993

961:                                              ; preds = %.loopexit89
  %962 = getelementptr i8, ptr %956, i64 -40
  %963 = load ptr, ptr %962, align 8, !noalias !44512, !nonnull !1708, !noundef !1708
  %964 = shl i64 %959, 3
  %965 = add i64 %964, -8
  %966 = load i64, ptr %932, align 8, !noalias !44513, !noundef !1708
  %967 = call i64 @llvm.umin.i64(i64 %965, i64 9223372036854775807)
  %968 = call i64 @llvm.ssub.sat.i64(i64 %966, i64 %967)
  store i64 %968, ptr %932, align 8, !noalias !44513
  %969 = load i64, ptr %933, align 8, !noalias !44513, !noundef !1708
  %970 = icmp slt i64 %968, %969
  br i1 %970, label %971, label %.preheader492

971:                                              ; preds = %961
  store i64 %968, ptr %933, align 8, !noalias !44513
  br label %.preheader492

.preheader492:                                    ; preds = %971, %961
  br label %972

972:                                              ; preds = %.preheader492, %975
  %973 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44513
  %974 = icmp slt i64 %973, 0
  br i1 %974, label %975, label %__rustc::__rust_dealloc (.exit75)

975:                                              ; preds = %972
  %976 = add nsw i64 %973, 1
  %977 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %973, i64 %976 acq_rel acquire, align 8, !noalias !44513
  %978 = extractvalue { i64, i1 } %977, 1
  br i1 %978, label %979, label %972

979:                                              ; preds = %975
  %980 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %967 monotonic, align 8, !noalias !44513
  %981 = call i64 @llvm.ssub.sat.i64(i64 %980, i64 %967)
  %982 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44513
  br label %983

983:                                              ; preds = %986, %979
  %984 = phi i64 [ %982, %979 ], [ %989, %986 ]
  %985 = icmp slt i64 %981, %984
  br i1 %985, label %986, label %990

986:                                              ; preds = %983
  %987 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %984, i64 %981 monotonic monotonic, align 8, !noalias !44513
  %988 = extractvalue { i64, i1 } %987, 1
  %989 = extractvalue { i64, i1 } %987, 0
  br i1 %988, label %990, label %983

990:                                              ; preds = %986, %983
  %991 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44513
  br label %__rustc::__rust_dealloc (.exit75)

__rustc::__rust_dealloc (.exit75): ; preds = %972, %990
  %992 = icmp ne i64 %965, 0
  call void @llvm.assume(i1 %992), !noalias !44513
  call void @free(ptr noundef nonnull %963) #88, !noalias !44513
  br label %993

993:                                              ; preds = %__rustc::__rust_dealloc (.exit75), %.loopexit89
  %994 = icmp eq i64 %957, 0
  br i1 %994, label %.loopexit91, label %934

.loopexit91:                                      ; preds = %993, %930
  %995 = icmp eq i64 %538, 0
  %996 = or i1 %527, %995
  br i1 %996, label %1296, label %997

997:                                              ; preds = %.loopexit91
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %539) ]
  %998 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %999 = load i64, ptr %998, align 8, !noalias !44516, !noundef !1708
  %1000 = call i64 @llvm.ssub.sat.i64(i64 %999, i64 %538)
  store i64 %1000, ptr %998, align 8, !noalias !44516
  %1001 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1002 = load i64, ptr %1001, align 8, !noalias !44516, !noundef !1708
  %1003 = icmp slt i64 %1000, %1002
  br i1 %1003, label %1004, label %.preheader491

1004:                                             ; preds = %997
  store i64 %1000, ptr %1001, align 8, !noalias !44516
  br label %.preheader491

.preheader491:                                    ; preds = %1004, %997
  br label %1005

1005:                                             ; preds = %.preheader491, %1008
  %1006 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44516
  %1007 = icmp slt i64 %1006, 0
  br i1 %1007, label %1008, label %__rustc::__rust_dealloc (.exit76)

1008:                                             ; preds = %1005
  %1009 = add nsw i64 %1006, 1
  %1010 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1006, i64 %1009 acq_rel acquire, align 8, !noalias !44516
  %1011 = extractvalue { i64, i1 } %1010, 1
  br i1 %1011, label %1012, label %1005

1012:                                             ; preds = %1008
  %1013 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %538 monotonic, align 8, !noalias !44516
  %1014 = call i64 @llvm.ssub.sat.i64(i64 %1013, i64 %538)
  %1015 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44516
  br label %1016

1016:                                             ; preds = %1019, %1012
  %1017 = phi i64 [ %1015, %1012 ], [ %1022, %1019 ]
  %1018 = icmp slt i64 %1014, %1017
  br i1 %1018, label %1019, label %1023

1019:                                             ; preds = %1016
  %1020 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1017, i64 %1014 monotonic monotonic, align 8, !noalias !44516
  %1021 = extractvalue { i64, i1 } %1020, 1
  %1022 = extractvalue { i64, i1 } %1020, 0
  br i1 %1021, label %1023, label %1016

1023:                                             ; preds = %1019, %1016
  %1024 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44516
  br label %__rustc::__rust_dealloc (.exit76)

__rustc::__rust_dealloc (.exit76): ; preds = %1005, %1023
  call void @free(ptr noundef nonnull %539) #88, !noalias !44516
  br label %1296

1025:                                             ; preds = %__rustc::__rust_dealloc (.exit71), %.loopexit87
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  br label %1086

1026:                                             ; preds = %__rustc::__rust_dealloc (.exit74), %.loopexit95
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !44384
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %15, ptr noundef nonnull align 8 dereferenceable(24) %10, i64 24, i1 false), !noalias !44388
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !44384
  %1027 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %1028 = load ptr, ptr %1027, align 8, !noalias !44304
  %1029 = getelementptr inbounds nuw i8, ptr %15, i64 16
  %1030 = load i64, ptr %1029, align 8, !noalias !44304
  call void @llvm.lifetime.end.p0(ptr nonnull %9)
  %1031 = icmp samesign ult i64 %1030, 2
  br i1 %1031, label %1086, label %1032, !prof !44517

1032:                                             ; preds = %1026
  %1033 = icmp samesign ult i64 %1030, 21
  br i1 %1033, label %1035, label %1034, !prof !1974

1034:                                             ; preds = %1032
; invoke core::slice::sort::unstable::ipnsort::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), <[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#1}>::{closure#0}>
  invoke void @core::slice::sort::unstable::ipnsort::<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), <[(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)]>::sort_unstable_by_key<usize, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#1}>::{closure#0}>(ptr noalias nofree noundef nonnull align 8 %1028, i64 noundef range(i64 0, 192153584101141163) %1030, ptr noalias nofree nonnull align 8 poison) #87
          to label %1086 unwind label %1234, !noalias !44304

1035:                                             ; preds = %1032
  %1036 = mul nuw nsw i64 %1030, 48
  %1037 = getelementptr inbounds nuw i8, ptr %1028, i64 %1036
  %1038 = getelementptr inbounds nuw i8, ptr %1028, i64 48
  %1039 = add nsw i64 %1036, -96
  %1040 = udiv i64 %1039, 48
  %1041 = and i64 %1040, 1
  %lcmp.mod554.not.not = icmp eq i64 %1041, 0
  br i1 %lcmp.mod554.not.not, label %.prol.preheader, label %.prol.loopexit

.prol.preheader:                                  ; preds = %1035
  %1042 = load i64, ptr %1038, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1043 = load i64, ptr %1028, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1044 = icmp ult i64 %1042, %1043
  br i1 %1044, label %._crit_edge.prol, label %.prol.loopexit.unr-lcssa

._crit_edge.prol:                                 ; preds = %.prol.preheader
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  %1045 = getelementptr inbounds nuw i8, ptr %1028, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %6, ptr noundef nonnull align 8 dereferenceable(40) %1045, i64 40, i1 false), !noalias !44304
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %1038, ptr noundef nonnull align 8 dereferenceable(48) %1028, i64 48, i1 false), !alias.scope !44518, !noalias !44304
  store i64 %1042, ptr %1028, align 8, !alias.scope !44518, !noalias !44523
  %1046 = getelementptr inbounds nuw i8, ptr %1028, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1046, ptr noundef nonnull align 8 dereferenceable(40) %6, i64 40, i1 false), !noalias !44523
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
  br label %.prol.loopexit.unr-lcssa

.prol.loopexit.unr-lcssa:                         ; preds = %._crit_edge.prol, %.prol.preheader
  %1047 = getelementptr inbounds nuw i8, ptr %1028, i64 96
  br label %.prol.loopexit

.prol.loopexit:                                   ; preds = %.prol.loopexit.unr-lcssa, %1035
  %.unr556 = phi ptr [ %1038, %1035 ], [ %1047, %.prol.loopexit.unr-lcssa ]
  %.unr557 = phi ptr [ %1028, %1035 ], [ %1038, %.prol.loopexit.unr-lcssa ]
  %1048 = icmp ult i64 %1039, 48
  br i1 %1048, label %.unr-lcssa, label %.new

.new:                                             ; preds = %.prol.loopexit, %1081
  %1049 = phi ptr [ %1082, %1081 ], [ %.unr556, %.prol.loopexit ]
  %1050 = phi ptr [ %1066, %1081 ], [ %.unr557, %.prol.loopexit ]
  %1051 = load i64, ptr %1049, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1052 = load i64, ptr %1050, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1053 = icmp ult i64 %1051, %1052
  br i1 %1053, label %1054, label %1065

1054:                                             ; preds = %.new
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  %1055 = getelementptr inbounds nuw i8, ptr %1050, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %6, ptr noundef nonnull align 8 dereferenceable(40) %1055, i64 40, i1 false), !noalias !44304
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %1049, ptr noundef nonnull align 8 dereferenceable(48) %1050, i64 48, i1 false), !alias.scope !44518, !noalias !44304
  %1056 = icmp eq ptr %1050, %1028
  br i1 %1056, label %._crit_edge, label %.lr.ph

1057:                                             ; preds = %.lr.ph
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %1059, ptr noundef nonnull align 8 dereferenceable(48) %1060, i64 48, i1 false), !alias.scope !44518, !noalias !44304
  %1058 = icmp eq ptr %1060, %1028
  br i1 %1058, label %._crit_edge, label %.lr.ph

.lr.ph:                                           ; preds = %1054, %1057
  %1059 = phi ptr [ %1060, %1057 ], [ %1050, %1054 ]
  %1060 = getelementptr inbounds i8, ptr %1059, i64 -48
  %1061 = load i64, ptr %1060, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1062 = icmp ult i64 %1051, %1061
  br i1 %1062, label %1057, label %._crit_edge

._crit_edge:                                      ; preds = %1057, %.lr.ph, %1054
  %1063 = phi ptr [ %1028, %1054 ], [ %1028, %1057 ], [ %1059, %.lr.ph ]
  store i64 %1051, ptr %1063, align 8, !alias.scope !44518, !noalias !44523
  %1064 = getelementptr inbounds nuw i8, ptr %1063, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1064, ptr noundef nonnull align 8 dereferenceable(40) %6, i64 40, i1 false), !noalias !44523
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
  br label %1065

1065:                                             ; preds = %._crit_edge, %.new
  %1066 = getelementptr inbounds nuw i8, ptr %1049, i64 48
  %1067 = load i64, ptr %1066, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1068 = load i64, ptr %1049, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1069 = icmp ult i64 %1067, %1068
  br i1 %1069, label %1070, label %1081

1070:                                             ; preds = %1065
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  %1071 = getelementptr inbounds nuw i8, ptr %1049, i64 56
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %6, ptr noundef nonnull align 8 dereferenceable(40) %1071, i64 40, i1 false), !noalias !44304
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %1066, ptr noundef nonnull align 8 dereferenceable(48) %1049, i64 48, i1 false), !alias.scope !44518, !noalias !44304
  %1072 = icmp eq ptr %1049, %1028
  br i1 %1072, label %._crit_edge.1, label %.lr.ph.1

.lr.ph.1:                                         ; preds = %1070, %1077
  %1073 = phi ptr [ %1074, %1077 ], [ %1049, %1070 ]
  %1074 = getelementptr inbounds i8, ptr %1073, i64 -48
  %1075 = load i64, ptr %1074, align 8, !alias.scope !44518, !noalias !44304, !noundef !1708
  %1076 = icmp ult i64 %1067, %1075
  br i1 %1076, label %1077, label %._crit_edge.1

1077:                                             ; preds = %.lr.ph.1
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(48) %1073, ptr noundef nonnull align 8 dereferenceable(48) %1074, i64 48, i1 false), !alias.scope !44518, !noalias !44304
  %1078 = icmp eq ptr %1074, %1028
  br i1 %1078, label %._crit_edge.1, label %.lr.ph.1

._crit_edge.1:                                    ; preds = %.lr.ph.1, %1077, %1070
  %1079 = phi ptr [ %1028, %1070 ], [ %1028, %1077 ], [ %1073, %.lr.ph.1 ]
  store i64 %1067, ptr %1079, align 8, !alias.scope !44518, !noalias !44523
  %1080 = getelementptr inbounds nuw i8, ptr %1079, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1080, ptr noundef nonnull align 8 dereferenceable(40) %6, i64 40, i1 false), !noalias !44523
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
  br label %1081

1081:                                             ; preds = %._crit_edge.1, %1065
  %1082 = getelementptr inbounds nuw i8, ptr %1049, i64 96
  %1083 = icmp eq ptr %1082, %1037
  br i1 %1083, label %.unr-lcssa, label %.new

.unr-lcssa:                                       ; preds = %1081, %.prol.loopexit
  %1084 = load ptr, ptr %1027, align 8, !noalias !44304
  %1085 = load i64, ptr %1029, align 8, !noalias !44304
  br label %1086

1086:                                             ; preds = %.unr-lcssa, %1034, %1026, %1025
  %1087 = phi i64 [ 0, %1025 ], [ %1030, %1034 ], [ %1030, %1026 ], [ %1085, %.unr-lcssa ]
  %1088 = phi ptr [ inttoptr (i64 8 to ptr), %1025 ], [ %1028, %1034 ], [ %1028, %1026 ], [ %1084, %.unr-lcssa ]
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !44304
  %1089 = getelementptr inbounds nuw i8, ptr %19, i64 24
  %1090 = load ptr, ptr %1089, align 8, !alias.scope !44302, !noalias !44299, !nonnull !1708, !noundef !1708
  store ptr %1090, ptr %14, align 8, !noalias !44304
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !44304
  %1091 = load i64, ptr %15, align 8, !range !1817, !noalias !44304, !noundef !1708
  %1092 = icmp ult i64 %1087, 192153584101141163
  call void @llvm.assume(i1 %1092)
  %1093 = mul nuw i64 %1087, 48
  %1094 = getelementptr inbounds nuw i8, ptr %1088, i64 %1093
  %1095 = getelementptr inbounds nuw i8, ptr %13, i64 8
  %1096 = getelementptr inbounds nuw i8, ptr %13, i64 16
  %1097 = getelementptr inbounds nuw i8, ptr %13, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !44528)
  call void @llvm.experimental.noalias.scope.decl(metadata !44531)
  %1098 = mul i64 %1091, 48
  %1099 = udiv i64 %1098, 40
  %1100 = icmp eq i64 %1087, 0
  br i1 %1100, label %.loopexit83, label %.preheader82.preheader

.preheader82.preheader:                           ; preds = %1086
  %1101 = add i64 %1093, -48
  %1102 = udiv i64 %1101, 48
  %1103 = add nuw nsw i64 %1102, 1
  %xtraiter558 = and i64 %1103, 7
  %lcmp.mod559.not = icmp eq i64 %xtraiter558, 0
  br i1 %lcmp.mod559.not, label %.preheader82.prol.loopexit, label %.preheader82.prol

.preheader82.prol:                                ; preds = %.preheader82.preheader, %.preheader82.prol
  %1104 = phi ptr [ %1108, %.preheader82.prol ], [ %1088, %.preheader82.preheader ]
  %1105 = phi ptr [ %1106, %.preheader82.prol ], [ %1088, %.preheader82.preheader ]
  %prol.iter560 = phi i64 [ %prol.iter560.next, %.preheader82.prol ], [ 0, %.preheader82.preheader ]
  %1106 = getelementptr inbounds nuw i8, ptr %1105, i64 48
  %1107 = getelementptr inbounds nuw i8, ptr %1105, i64 8
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1104, ptr noundef nonnull align 8 dereferenceable(40) %1107, i64 40, i1 false), !noalias !44534
  %1108 = getelementptr inbounds nuw i8, ptr %1104, i64 40
  %prol.iter560.next = add i64 %prol.iter560, 1
  %prol.iter560.cmp.not = icmp eq i64 %prol.iter560.next, %xtraiter558
  br i1 %prol.iter560.cmp.not, label %.preheader82.prol.loopexit, label %.preheader82.prol, !llvm.loop !44543

.preheader82.prol.loopexit:                       ; preds = %.preheader82.prol, %.preheader82.preheader
  %.lcssa.unr = phi ptr [ poison, %.preheader82.preheader ], [ %1108, %.preheader82.prol ]
  %.unr561 = phi ptr [ %1088, %.preheader82.preheader ], [ %1108, %.preheader82.prol ]
  %.unr562 = phi ptr [ %1088, %.preheader82.preheader ], [ %1106, %.preheader82.prol ]
  %1109 = icmp ult i64 %1101, 336
  br i1 %1109, label %.loopexit83, label %.preheader82

.preheader82:                                     ; preds = %.preheader82.prol.loopexit, %.preheader82
  %1110 = phi ptr [ %1128, %.preheader82 ], [ %.unr561, %.preheader82.prol.loopexit ]
  %1111 = phi ptr [ %1126, %.preheader82 ], [ %.unr562, %.preheader82.prol.loopexit ]
  %1112 = getelementptr inbounds nuw i8, ptr %1111, i64 8
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1110, ptr noundef nonnull align 8 dereferenceable(40) %1112, i64 40, i1 false), !noalias !44534
  %1113 = getelementptr inbounds nuw i8, ptr %1110, i64 40
  %1114 = getelementptr inbounds nuw i8, ptr %1111, i64 56
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1113, ptr noundef nonnull align 8 dereferenceable(40) %1114, i64 40, i1 false), !noalias !44534
  %1115 = getelementptr inbounds nuw i8, ptr %1110, i64 80
  %1116 = getelementptr inbounds nuw i8, ptr %1111, i64 104
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1115, ptr noundef nonnull align 8 dereferenceable(40) %1116, i64 40, i1 false), !noalias !44534
  %1117 = getelementptr inbounds nuw i8, ptr %1110, i64 120
  %1118 = getelementptr inbounds nuw i8, ptr %1111, i64 152
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1117, ptr noundef nonnull align 8 dereferenceable(40) %1118, i64 40, i1 false), !noalias !44534
  %1119 = getelementptr inbounds nuw i8, ptr %1110, i64 160
  %1120 = getelementptr inbounds nuw i8, ptr %1111, i64 200
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1119, ptr noundef nonnull align 8 dereferenceable(40) %1120, i64 40, i1 false), !noalias !44534
  %1121 = getelementptr inbounds nuw i8, ptr %1110, i64 200
  %1122 = getelementptr inbounds nuw i8, ptr %1111, i64 248
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1121, ptr noundef nonnull align 8 dereferenceable(40) %1122, i64 40, i1 false), !noalias !44534
  %1123 = getelementptr inbounds nuw i8, ptr %1110, i64 240
  %1124 = getelementptr inbounds nuw i8, ptr %1111, i64 296
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1123, ptr noundef nonnull align 8 dereferenceable(40) %1124, i64 40, i1 false), !noalias !44534
  %1125 = getelementptr inbounds nuw i8, ptr %1110, i64 280
  %1126 = getelementptr inbounds nuw i8, ptr %1111, i64 384
  %1127 = getelementptr inbounds nuw i8, ptr %1111, i64 344
  call void @llvm.memmove.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %1125, ptr noundef nonnull align 8 dereferenceable(40) %1127, i64 40, i1 false), !noalias !44534
  %1128 = getelementptr inbounds nuw i8, ptr %1110, i64 320
  %1129 = icmp eq ptr %1126, %1094
  br i1 %1129, label %.loopexit83, label %.preheader82

.loopexit83:                                      ; preds = %.preheader82.prol.loopexit, %.preheader82, %1086
  %1130 = phi ptr [ %1088, %1086 ], [ %1094, %.preheader82 ], [ %1094, %.preheader82.prol.loopexit ]
  %1131 = phi ptr [ %1088, %1086 ], [ %.lcssa.unr, %.preheader82.prol.loopexit ], [ %1128, %.preheader82 ]
  %1132 = ptrtoint ptr %1131 to i64
  %1133 = ptrtoint ptr %1088 to i64
  %1134 = sub nuw i64 %1132, %1133
  %1135 = udiv exact i64 %1134, 40
  call void @llvm.lifetime.start.p0(ptr nonnull %5), !noalias !44544
  store ptr %1088, ptr %5, align 8, !noalias !44544
  %1136 = getelementptr inbounds nuw i8, ptr %5, i64 8
  store i64 %1135, ptr %1136, align 8, !noalias !44544
  %1137 = getelementptr inbounds nuw i8, ptr %5, i64 16
  store i64 %1091, ptr %1137, align 8, !noalias !44544
  call void @llvm.experimental.noalias.scope.decl(metadata !44545)
  %1138 = ptrtoint ptr %1094 to i64
  %1139 = ptrtoint ptr %1130 to i64
  %1140 = sub nuw i64 %1138, %1139
  %1141 = udiv exact i64 %1140, 48
  store i64 0, ptr %1096, align 8, !alias.scope !44548, !noalias !44549
  store ptr inttoptr (i64 8 to ptr), ptr %13, align 8, !alias.scope !44548, !noalias !44549
  store ptr inttoptr (i64 8 to ptr), ptr %1095, align 8, !alias.scope !44548, !noalias !44549
  store ptr inttoptr (i64 8 to ptr), ptr %1097, align 8, !alias.scope !44548, !noalias !44549
  call void @llvm.experimental.noalias.scope.decl(metadata !44550)
  %1142 = icmp eq ptr %1094, %1130
  br i1 %1142, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %.loopexit83
  %1143 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1144 = call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  br label %1145

1145:                                             ; preds = %.preheader, %1184
  %1146 = phi i64 [ %1148, %1184 ], [ 0, %.preheader ]
  %1147 = getelementptr inbounds nuw [48 x i8], ptr %1130, i64 %1146
  %1148 = add nuw nsw i64 %1146, 1
  %1149 = getelementptr i8, ptr %1147, i64 8
  %1150 = load i64, ptr %1149, align 8, !range !1940, !alias.scope !44553, !noalias !44556, !noundef !1708
  %1151 = icmp ugt i64 %1150, 5
  br i1 %1151, label %1152, label %1184

1152:                                             ; preds = %1145
  %1153 = getelementptr i8, ptr %1147, i64 16
  %1154 = load ptr, ptr %1153, align 8, !alias.scope !44550, !noalias !44556, !nonnull !1708, !noundef !1708
  %1155 = shl i64 %1150, 3
  %1156 = add i64 %1155, -8
  %1157 = load i64, ptr %1143, align 8, !noalias !44557, !noundef !1708
  %1158 = call i64 @llvm.umin.i64(i64 %1156, i64 9223372036854775807)
  %1159 = call i64 @llvm.ssub.sat.i64(i64 %1157, i64 %1158)
  store i64 %1159, ptr %1143, align 8, !noalias !44557
  %1160 = load i64, ptr %1144, align 8, !noalias !44557, !noundef !1708
  %1161 = icmp slt i64 %1159, %1160
  br i1 %1161, label %1162, label %.preheader485

1162:                                             ; preds = %1152
  store i64 %1159, ptr %1144, align 8, !noalias !44557
  br label %.preheader485

.preheader485:                                    ; preds = %1162, %1152
  br label %1163

1163:                                             ; preds = %.preheader485, %1166
  %1164 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44557
  %1165 = icmp slt i64 %1164, 0
  br i1 %1165, label %1166, label %__rustc::__rust_dealloc (.exit77)

1166:                                             ; preds = %1163
  %1167 = add nsw i64 %1164, 1
  %1168 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1164, i64 %1167 acq_rel acquire, align 8, !noalias !44557
  %1169 = extractvalue { i64, i1 } %1168, 1
  br i1 %1169, label %1170, label %1163

1170:                                             ; preds = %1166
  %1171 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1158 monotonic, align 8, !noalias !44557
  %1172 = call i64 @llvm.ssub.sat.i64(i64 %1171, i64 %1158)
  %1173 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44557
  br label %1174

1174:                                             ; preds = %1177, %1170
  %1175 = phi i64 [ %1173, %1170 ], [ %1180, %1177 ]
  %1176 = icmp slt i64 %1172, %1175
  br i1 %1176, label %1177, label %1181

1177:                                             ; preds = %1174
  %1178 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1175, i64 %1172 monotonic monotonic, align 8, !noalias !44557
  %1179 = extractvalue { i64, i1 } %1178, 1
  %1180 = extractvalue { i64, i1 } %1178, 0
  br i1 %1179, label %1181, label %1174

1181:                                             ; preds = %1177, %1174
  %1182 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44557
  br label %__rustc::__rust_dealloc (.exit77)

__rustc::__rust_dealloc (.exit77): ; preds = %1163, %1181
  %1183 = icmp ne i64 %1156, 0
  call void @llvm.assume(i1 %1183), !noalias !44557
  call void @free(ptr noundef nonnull %1154) #88, !noalias !44557
  br label %1184

1184:                                             ; preds = %__rustc::__rust_dealloc (.exit77), %1145
  %1185 = icmp eq i64 %1148, %1141
  br i1 %1185, label %.loopexit, label %1145

1186:                                             ; preds = %1229
  %1187 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::in_place_drop::InPlaceDstDataSrcBufDrop<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>), purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %5) #89, !noalias !44544
; call core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#2}>>
  call fastcc void @core::ptr::drop_glue::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>, purrdf_sparql_eval::modifier::dedup<purrdf_core::ir::term::TermId>::{closure#2}>>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %13) #89, !noalias !44549
  %1188 = atomicrmw sub ptr %1090, i64 1 release, align 8, !noalias !44560
  %1189 = icmp eq i64 %1188, 1
  br i1 %1189, label %1231, label %1364

.loopexit:                                        ; preds = %1184, %.loopexit83
  %1190 = icmp ne i64 %1091, 0
  %1191 = mul nuw i64 %1099, 40
  %1192 = icmp ne i64 %1098, %1191
  %1193 = select i1 %1190, i1 %1192, i1 false
  br i1 %1193, label %1194, label %1305

1194:                                             ; preds = %.loopexit
  %1195 = icmp ult i64 %1098, 40
  br i1 %1195, label %1196, label %1226

1196:                                             ; preds = %1194
  %1197 = icmp eq i64 %1098, 0
  br i1 %1197, label %1305, label %1198

1198:                                             ; preds = %1196
  %1199 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1200 = load i64, ptr %1199, align 8, !noalias !44544, !noundef !1708
  %1201 = call i64 @llvm.ssub.sat.i64(i64 %1200, i64 %1098)
  store i64 %1201, ptr %1199, align 8, !noalias !44544
  %1202 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1203 = load i64, ptr %1202, align 8, !noalias !44544, !noundef !1708
  %1204 = icmp slt i64 %1201, %1203
  br i1 %1204, label %1205, label %.preheader482

1205:                                             ; preds = %1198
  store i64 %1201, ptr %1202, align 8, !noalias !44544
  br label %.preheader482

.preheader482:                                    ; preds = %1205, %1198
  br label %1206

1206:                                             ; preds = %.preheader482, %1209
  %1207 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44544
  %1208 = icmp slt i64 %1207, 0
  br i1 %1208, label %1209, label %__rustc::__rust_dealloc (.exit78)

1209:                                             ; preds = %1206
  %1210 = add nsw i64 %1207, 1
  %1211 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1207, i64 %1210 acq_rel acquire, align 8, !noalias !44544
  %1212 = extractvalue { i64, i1 } %1211, 1
  br i1 %1212, label %1213, label %1206

1213:                                             ; preds = %1209
  %1214 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1098 monotonic, align 8, !noalias !44544
  %1215 = call i64 @llvm.ssub.sat.i64(i64 %1214, i64 %1098)
  %1216 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44544
  br label %1217

1217:                                             ; preds = %1220, %1213
  %1218 = phi i64 [ %1216, %1213 ], [ %1223, %1220 ]
  %1219 = icmp slt i64 %1215, %1218
  br i1 %1219, label %1220, label %1224

1220:                                             ; preds = %1217
  %1221 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1218, i64 %1215 monotonic monotonic, align 8, !noalias !44544
  %1222 = extractvalue { i64, i1 } %1221, 1
  %1223 = extractvalue { i64, i1 } %1221, 0
  br i1 %1222, label %1224, label %1217

1224:                                             ; preds = %1220, %1217
  %1225 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44544
  br label %__rustc::__rust_dealloc (.exit78)

__rustc::__rust_dealloc (.exit78): ; preds = %1206, %1224
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1088) ], !noalias !44544
  call void @free(ptr noundef nonnull %1088) #88, !noalias !44544
  br label %1305

1226:                                             ; preds = %1194
  %1227 = icmp ule i64 %1191, %1098
  call void @llvm.assume(i1 %1227)
; call <purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc
  %_0.i = call noalias noundef align 8 ptr @<purrdf_alloc_probe::CountingAllocator as core::alloc::global::GlobalAlloc>::realloc(ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @qualification_454_native_cost::GLOBAL (.llvm.11174181910260379007), ptr noundef nonnull %1088, i64 noundef 8, i64 noundef %1098, i64 noundef %1191) #88, !noalias !44544
  %1228 = icmp eq ptr %_0.i, null
  br i1 %1228, label %1229, label %1305, !prof !4226

1229:                                             ; preds = %1226
; invoke alloc::alloc::handle_alloc_error
  invoke void @alloc::alloc::handle_alloc_error(i64 noundef 8, i64 noundef %1191) #90
          to label %1230 unwind label %1186, !noalias !44544

1230:                                             ; preds = %1229
  unreachable

1231:                                             ; preds = %1186
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %14) #87
          to label %1364 unwind label %1232, !noalias !44304

1232:                                             ; preds = %1303, %1231
  %1233 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86, !noalias !44304
  unreachable

1234:                                             ; preds = %1034
  %1235 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(usize, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %15) #89, !noalias !44304
  br label %1296

1236:                                             ; preds = %438, %.loopexit115
  %1237 = phi i64 [ %310, %.loopexit115 ], [ %440, %438 ]
  %1238 = phi ptr [ %311, %.loopexit115 ], [ %439, %438 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !44348)
  %1239 = and i64 %1237, %307
  %1240 = getelementptr inbounds nuw i8, ptr %1238, i64 %1239
  %1241 = load <16 x i8>, ptr %1240, align 1, !noalias !44565
  %1242 = icmp slt <16 x i8> %1241, zeroinitializer
  %1243 = bitcast <16 x i1> %1242 to i16
  %1244 = icmp eq i16 %1243, 0
  br i1 %1244, label %.preheader112, label %.loopexit113, !prof !5062

.loopexit113:                                     ; preds = %.preheader112, %1236
  %1245 = phi i64 [ %1239, %1236 ], [ %1267, %.preheader112 ]
  %1246 = phi i16 [ %1243, %1236 ], [ %1271, %.preheader112 ]
  %1247 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %1246, i1 true)
  %1248 = zext nneg i16 %1247 to i64
  %1249 = add i64 %1245, %1248
  %1250 = and i64 %1249, %1237
  %1251 = getelementptr inbounds nuw i8, ptr %1238, i64 %1250
  %1252 = load i8, ptr %1251, align 1, !noalias !44568, !noundef !1708
  %1253 = icmp sgt i8 %1252, -1
  br i1 %1253, label %1254, label %1275, !prof !1803

1254:                                             ; preds = %.loopexit113
  %1255 = load <16 x i8>, ptr %1238, align 16, !noalias !44568
  %1256 = icmp slt <16 x i8> %1255, zeroinitializer
  %1257 = bitcast <16 x i1> %1256 to i16
  %1258 = icmp ne i16 %1257, 0
  %1259 = call range(i16 0, 17) i16 @llvm.cttz.i16(i16 %1257, i1 true)
  %1260 = zext nneg i16 %1259 to i64
  call void @llvm.assume(i1 %1258)
  %1261 = getelementptr inbounds nuw i8, ptr %1238, i64 %1260
  %1262 = load i8, ptr %1261, align 1, !noalias !44568
  br label %1275

.preheader112:                                    ; preds = %1236, %.preheader112
  %1263 = phi i64 [ %1267, %.preheader112 ], [ %1239, %1236 ]
  %1264 = phi i64 [ %1265, %.preheader112 ], [ 0, %1236 ]
  %1265 = add i64 %1264, 16
  %1266 = add i64 %1265, %1263
  %1267 = and i64 %1266, %1237
  %1268 = getelementptr inbounds nuw i8, ptr %1238, i64 %1267
  %1269 = load <16 x i8>, ptr %1268, align 1, !noalias !44565
  %1270 = icmp slt <16 x i8> %1269, zeroinitializer
  %1271 = bitcast <16 x i1> %1270 to i16
  %1272 = icmp eq i16 %1271, 0
  br i1 %1272, label %.preheader112, label %.loopexit113, !prof !5063

1273:                                             ; preds = %1275, %__rustc::__rust_dealloc (.exit67), %.loopexit106
  %1274 = icmp eq ptr %151, %134
  br i1 %1274, label %.loopexit103, label %148

1275:                                             ; preds = %1254, %.loopexit113
  %1276 = phi i8 [ %1262, %1254 ], [ %1252, %.loopexit113 ]
  %1277 = phi i64 [ %1260, %1254 ], [ %1250, %.loopexit113 ]
  %1278 = getelementptr inbounds nuw i8, ptr %1238, i64 %1277
  %1279 = add i64 %1277, -16
  %1280 = and i64 %1279, %1237
  store i8 %309, ptr %1278, align 1, !noalias !44568
  %1281 = getelementptr i8, ptr %1238, i64 %1280
  %1282 = getelementptr i8, ptr %1281, i64 16
  store i8 %309, ptr %1282, align 1, !noalias !44568
  %1283 = sub nsw i64 0, %1277
  %1284 = getelementptr inbounds [48 x i8], ptr %1238, i64 %1283
  %1285 = and i8 %1276, 1
  %1286 = zext nneg i8 %1285 to i64
  %1287 = getelementptr inbounds i8, ptr %1284, i64 -48
  store i64 %152, ptr %1287, align 8, !noalias !44569
  %1288 = getelementptr inbounds i8, ptr %1284, i64 -40
  store ptr %160, ptr %1288, align 8, !noalias !44569
  %1289 = getelementptr inbounds i8, ptr %1284, i64 -32
  store ptr %171, ptr %1289, align 8, !noalias !44569
  %1290 = getelementptr inbounds i8, ptr %1284, i64 -24
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(16) %1290, ptr noundef nonnull align 8 dereferenceable(16) %145, i64 16, i1 false), !noalias !44304
  %1291 = getelementptr inbounds i8, ptr %1284, i64 -8
  store i64 %150, ptr %1291, align 8, !noalias !44569
  %1292 = load <2 x i64>, ptr %144, align 8, !alias.scope !44348, !noalias !44351
  %1293 = insertelement <2 x i64> <i64 poison, i64 -1>, i64 %1286, i64 0
  %1294 = sub <2 x i64> %1292, %1293
  store <2 x i64> %1294, ptr %144, align 8, !alias.scope !44348, !noalias !44351
  br label %1273

1295:                                             ; preds = %1303, %1296
  br i1 %1298, label %1304, label %1364

1296:                                             ; preds = %1234, %__rustc::__rust_dealloc (.exit76), %.loopexit91, %807, %154, %127
  %1297 = phi { ptr, i32 } [ %128, %127 ], [ %299, %154 ], [ %672, %__rustc::__rust_dealloc (.exit76) ], [ %672, %.loopexit91 ], [ %921, %807 ], [ %1235, %1234 ]
  %1298 = phi i1 [ true, %127 ], [ false, %154 ], [ false, %__rustc::__rust_dealloc (.exit76) ], [ false, %.loopexit91 ], [ false, %807 ], [ false, %1234 ]
  %1299 = getelementptr inbounds nuw i8, ptr %19, i64 24
  call void @llvm.experimental.noalias.scope.decl(metadata !44570)
  call void @llvm.experimental.noalias.scope.decl(metadata !44573)
  %1300 = load ptr, ptr %1299, align 8, !alias.scope !44576, !noalias !44299, !nonnull !1708, !noundef !1708
  %1301 = atomicrmw sub ptr %1300, i64 1 release, align 8, !noalias !44577
  %1302 = icmp eq i64 %1301, 1
  br i1 %1302, label %1303, label %1295

1303:                                             ; preds = %1296
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1299) #87
          to label %1295 unwind label %1232, !noalias !44299

1304:                                             ; preds = %1295
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %19) #89, !noalias !44299
  br label %1364

1305:                                             ; preds = %1226, %__rustc::__rust_dealloc (.exit78), %1196, %.loopexit
  %1306 = phi ptr [ inttoptr (i64 8 to ptr), %1196 ], [ %_0.i, %1226 ], [ %1088, %.loopexit ], [ inttoptr (i64 8 to ptr), %__rustc::__rust_dealloc (.exit78) ]
  call void @llvm.lifetime.end.p0(ptr nonnull %5), !noalias !44544
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !44304
  %1307 = getelementptr inbounds nuw i8, ptr %20, i64 24
  store ptr %1090, ptr %1307, align 8, !alias.scope !44299, !noalias !44302
  store i64 %1099, ptr %20, align 8, !alias.scope !44299, !noalias !44302
  %1308 = getelementptr inbounds nuw i8, ptr %20, i64 8
  store ptr %1306, ptr %1308, align 8, !alias.scope !44299, !noalias !44302
  %1309 = getelementptr inbounds nuw i8, ptr %20, i64 16
  store i64 %1135, ptr %1309, align 8, !alias.scope !44299, !noalias !44302
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !44304
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !44304
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !44304
  call void @llvm.lifetime.end.p0(ptr nonnull %11)
  call void @llvm.lifetime.end.p0(ptr nonnull %19)
  call void @llvm.experimental.noalias.scope.decl(metadata !44578)
  call void @llvm.experimental.noalias.scope.decl(metadata !44581)
  call void @llvm.experimental.noalias.scope.decl(metadata !44583)
  %1310 = load i64, ptr %21, align 8, !range !2062, !alias.scope !44581, !noalias !44585, !noundef !1708
  %1311 = icmp eq i64 %1310, -1
  br i1 %1311, label %1313, label %1312

1312:                                             ; preds = %1305
; call <purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new
  call fastcc void @<purrdf_sparql_eval::governor::lift::Truncation<purrdf_core::ir::term::TermId>>::new(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(96) %22, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(32) %20, ptr noalias nofree noundef nonnull readonly align 8 captures(address) dereferenceable(104) %21)
  br label %1315

1313:                                             ; preds = %1305
  %1314 = getelementptr inbounds nuw i8, ptr %22, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %1314, ptr noundef nonnull readonly align 8 dereferenceable(32) %20, i64 32, i1 false), !alias.scope !44585, !noalias !44581
  store i64 -1, ptr %22, align 8, !alias.scope !44578, !noalias !44586
  br label %1315

1315:                                             ; preds = %1313, %1312
  %1316 = getelementptr inbounds nuw i8, ptr %21, i64 72
  %1317 = load i64, ptr %1316, align 8, !range !1940, !alias.scope !44587, !noalias !44585, !noundef !1708
  %1318 = icmp ugt i64 %1317, 5
  br i1 %1318, label %1319, label %1353

1319:                                             ; preds = %1315
  %1320 = getelementptr inbounds nuw i8, ptr %21, i64 80
  %1321 = load ptr, ptr %1320, align 8, !alias.scope !44581, !noalias !44585, !nonnull !1708, !noundef !1708
  %1322 = mul i64 %1317, 3
  %1323 = add i64 %1322, -3
  %1324 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1325 = load i64, ptr %1324, align 8, !noalias !44590, !noundef !1708
  %1326 = call i64 @llvm.umin.i64(i64 %1323, i64 9223372036854775807)
  %1327 = call i64 @llvm.ssub.sat.i64(i64 %1325, i64 %1326)
  store i64 %1327, ptr %1324, align 8, !noalias !44590
  %1328 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1329 = load i64, ptr %1328, align 8, !noalias !44590, !noundef !1708
  %1330 = icmp slt i64 %1327, %1329
  br i1 %1330, label %1331, label %.preheader481

1331:                                             ; preds = %1319
  store i64 %1327, ptr %1328, align 8, !noalias !44590
  br label %.preheader481

.preheader481:                                    ; preds = %1331, %1319
  br label %1332

1332:                                             ; preds = %.preheader481, %1335
  %1333 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44590
  %1334 = icmp slt i64 %1333, 0
  br i1 %1334, label %1335, label %__rustc::__rust_dealloc (.exit79)

1335:                                             ; preds = %1332
  %1336 = add nsw i64 %1333, 1
  %1337 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1333, i64 %1336 acq_rel acquire, align 8, !noalias !44590
  %1338 = extractvalue { i64, i1 } %1337, 1
  br i1 %1338, label %1339, label %1332

1339:                                             ; preds = %1335
  %1340 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1326 monotonic, align 8, !noalias !44590
  %1341 = call i64 @llvm.ssub.sat.i64(i64 %1340, i64 %1326)
  %1342 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44590
  br label %1343

1343:                                             ; preds = %1346, %1339
  %1344 = phi i64 [ %1342, %1339 ], [ %1349, %1346 ]
  %1345 = icmp slt i64 %1341, %1344
  br i1 %1345, label %1346, label %1350

1346:                                             ; preds = %1343
  %1347 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1344, i64 %1341 monotonic monotonic, align 8, !noalias !44590
  %1348 = extractvalue { i64, i1 } %1347, 1
  %1349 = extractvalue { i64, i1 } %1347, 0
  br i1 %1348, label %1350, label %1343

1350:                                             ; preds = %1346, %1343
  %1351 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44590
  br label %__rustc::__rust_dealloc (.exit79)

__rustc::__rust_dealloc (.exit79): ; preds = %1332, %1350
  %1352 = icmp ne i64 %1323, 0
  call void @llvm.assume(i1 %1352), !noalias !44590
  call void @free(ptr noundef nonnull %1321) #88, !noalias !44590
  br label %1353

1353:                                             ; preds = %__rustc::__rust_dealloc (.exit79), %1315
  %1354 = getelementptr inbounds nuw i8, ptr %21, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !44593)
  %1355 = load ptr, ptr %1354, align 8, !alias.scope !44596, !noalias !44585, !noundef !1708
  %1356 = icmp eq ptr %1355, null
  br i1 %1356, label %1361, label %1357

1357:                                             ; preds = %1353
  %1358 = atomicrmw sub ptr %1355, i64 1 release, align 8, !noalias !44597
  %1359 = icmp eq i64 %1358, 1
  br i1 %1359, label %1360, label %1361

1360:                                             ; preds = %1357
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1354) #87
  br label %1361

1361:                                             ; preds = %1360, %1357, %1353
  call void @llvm.lifetime.end.p0(ptr nonnull %20)
  call void @llvm.lifetime.end.p0(ptr nonnull %21)
  %1362 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(96) %1362, ptr noundef nonnull align 8 dereferenceable(96) %22, i64 96, i1 false)
  store i64 0, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %22)
  call void @llvm.lifetime.end.p0(ptr nonnull %25)
  br label %1363

1363:                                             ; preds = %1361, %121, %112, %109, %105
  ret void

1364:                                             ; preds = %1304, %1295, %1231, %1186, %119
  %1365 = phi { ptr, i32 } [ %120, %119 ], [ %1297, %1295 ], [ %1297, %1304 ], [ %1187, %1231 ], [ %1187, %1186 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !44602)
  %1366 = getelementptr inbounds nuw i8, ptr %21, i64 72
  %1367 = load i64, ptr %1366, align 8, !range !1940, !alias.scope !44605, !noundef !1708
  %1368 = icmp ugt i64 %1367, 5
  br i1 %1368, label %1369, label %1403

1369:                                             ; preds = %1364
  %1370 = getelementptr inbounds nuw i8, ptr %21, i64 80
  %1371 = load ptr, ptr %1370, align 8, !alias.scope !44602, !nonnull !1708, !noundef !1708
  %1372 = mul i64 %1367, 3
  %1373 = add i64 %1372, -3
  %1374 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1375 = load i64, ptr %1374, align 8, !noalias !44608, !noundef !1708
  %1376 = call i64 @llvm.umin.i64(i64 %1373, i64 9223372036854775807)
  %1377 = call i64 @llvm.ssub.sat.i64(i64 %1375, i64 %1376)
  store i64 %1377, ptr %1374, align 8, !noalias !44608
  %1378 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1379 = load i64, ptr %1378, align 8, !noalias !44608, !noundef !1708
  %1380 = icmp slt i64 %1377, %1379
  br i1 %1380, label %1381, label %.preheader484

1381:                                             ; preds = %1369
  store i64 %1377, ptr %1378, align 8, !noalias !44608
  br label %.preheader484

.preheader484:                                    ; preds = %1381, %1369
  br label %1382

1382:                                             ; preds = %.preheader484, %1385
  %1383 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44608
  %1384 = icmp slt i64 %1383, 0
  br i1 %1384, label %1385, label %__rustc::__rust_dealloc (.exit80)

1385:                                             ; preds = %1382
  %1386 = add nsw i64 %1383, 1
  %1387 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1383, i64 %1386 acq_rel acquire, align 8, !noalias !44608
  %1388 = extractvalue { i64, i1 } %1387, 1
  br i1 %1388, label %1389, label %1382

1389:                                             ; preds = %1385
  %1390 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1376 monotonic, align 8, !noalias !44608
  %1391 = call i64 @llvm.ssub.sat.i64(i64 %1390, i64 %1376)
  %1392 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44608
  br label %1393

1393:                                             ; preds = %1396, %1389
  %1394 = phi i64 [ %1392, %1389 ], [ %1399, %1396 ]
  %1395 = icmp slt i64 %1391, %1394
  br i1 %1395, label %1396, label %1400

1396:                                             ; preds = %1393
  %1397 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1394, i64 %1391 monotonic monotonic, align 8, !noalias !44608
  %1398 = extractvalue { i64, i1 } %1397, 1
  %1399 = extractvalue { i64, i1 } %1397, 0
  br i1 %1398, label %1400, label %1393

1400:                                             ; preds = %1396, %1393
  %1401 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44608
  br label %__rustc::__rust_dealloc (.exit80)

__rustc::__rust_dealloc (.exit80): ; preds = %1382, %1400
  %1402 = icmp ne i64 %1373, 0
  call void @llvm.assume(i1 %1402), !noalias !44608
  call void @free(ptr noundef nonnull %1371) #88, !noalias !44608
  br label %1403

1403:                                             ; preds = %__rustc::__rust_dealloc (.exit80), %1364
  %1404 = load i64, ptr %21, align 8, !range !2062, !alias.scope !44602, !noundef !1708
  %1405 = icmp sgt i64 %1404, 0
  br i1 %1405, label %1406, label %1438

1406:                                             ; preds = %1403
  %1407 = getelementptr inbounds nuw i8, ptr %21, i64 8
  %1408 = load ptr, ptr %1407, align 8, !alias.scope !44602, !nonnull !1708, !noundef !1708
  %1409 = mul nuw i64 %1404, 3
  %1410 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1411 = load i64, ptr %1410, align 8, !noalias !44602, !noundef !1708
  %1412 = call i64 @llvm.umin.i64(i64 %1409, i64 9223372036854775807)
  %1413 = call i64 @llvm.ssub.sat.i64(i64 %1411, i64 %1412)
  store i64 %1413, ptr %1410, align 8, !noalias !44602
  %1414 = call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.11284292287820896952))
  %1415 = load i64, ptr %1414, align 8, !noalias !44602, !noundef !1708
  %1416 = icmp slt i64 %1413, %1415
  br i1 %1416, label %1417, label %.preheader483

1417:                                             ; preds = %1406
  store i64 %1413, ptr %1414, align 8, !noalias !44602
  br label %.preheader483

.preheader483:                                    ; preds = %1417, %1406
  br label %1418

1418:                                             ; preds = %.preheader483, %1421
  %1419 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952) acquire, align 8, !noalias !44602
  %1420 = icmp slt i64 %1419, 0
  br i1 %1420, label %1421, label %__rustc::__rust_dealloc (.exit81)

1421:                                             ; preds = %1418
  %1422 = add nsw i64 %1419, 1
  %1423 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 %1419, i64 %1422 acq_rel acquire, align 8, !noalias !44602
  %1424 = extractvalue { i64, i1 } %1423, 1
  br i1 %1424, label %1425, label %1418

1425:                                             ; preds = %1421
  %1426 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1412 monotonic, align 8, !noalias !44602
  %1427 = call i64 @llvm.ssub.sat.i64(i64 %1426, i64 %1412)
  %1428 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !44602
  br label %1429

1429:                                             ; preds = %1432, %1425
  %1430 = phi i64 [ %1428, %1425 ], [ %1435, %1432 ]
  %1431 = icmp slt i64 %1427, %1430
  br i1 %1431, label %1432, label %1436

1432:                                             ; preds = %1429
  %1433 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1430, i64 %1427 monotonic monotonic, align 8, !noalias !44602
  %1434 = extractvalue { i64, i1 } %1433, 1
  %1435 = extractvalue { i64, i1 } %1433, 0
  br i1 %1434, label %1436, label %1429

1436:                                             ; preds = %1432, %1429
  %1437 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.11284292287820896952), i64 1 release, align 8, !noalias !44602
  br label %__rustc::__rust_dealloc (.exit81)

__rustc::__rust_dealloc (.exit81): ; preds = %1418, %1436
  call void @free(ptr noundef nonnull %1408) #88, !noalias !44602
  br label %1438

1438:                                             ; preds = %__rustc::__rust_dealloc (.exit81), %1403
  %1439 = getelementptr inbounds nuw i8, ptr %21, i64 96
  call void @llvm.experimental.noalias.scope.decl(metadata !44611)
  %1440 = load ptr, ptr %1439, align 8, !alias.scope !44614, !noundef !1708
  %1441 = icmp eq ptr %1440, null
  br i1 %1441, label %1448, label %1442

1442:                                             ; preds = %1438
  %1443 = atomicrmw sub ptr %1440, i64 1 release, align 8, !noalias !44615
  %1444 = icmp eq i64 %1443, 1
  br i1 %1444, label %1445, label %1448

1445:                                             ; preds = %1442
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::solution::VarSchema>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %1439) #87
          to label %1448 unwind label %1446

1446:                                             ; preds = %1450, %1445
  %1447 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #86
  unreachable

1448:                                             ; preds = %1450, %1445, %1442, %1438
  %1449 = phi { ptr, i32 } [ %1451, %1450 ], [ %1365, %1445 ], [ %1365, %1438 ], [ %1365, %1442 ]
  resume { ptr, i32 } %1449

1450:                                             ; preds = %113, %4
  %1451 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::governor::lift::Lift>(ptr noalias nofree noundef align 8 dereferenceable(104) %26) #89
          to label %1448 unwind label %1446
}
