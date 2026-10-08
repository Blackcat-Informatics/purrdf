define internal fastcc void @<purrdf_sparql_eval::row_checkpoint::Committing>::of::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>::{closure#1}>>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 8 captures(none) dereferenceable(32) %0, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(32) %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !6121 {
  %3 = alloca [160 x i8], align 8
  %4 = alloca [32 x i8], align 8
  %5 = alloca [24 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %5)
  store i64 0, ptr %5, align 8
  %6 = getelementptr inbounds nuw i8, ptr %5, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %6, align 8
  %7 = getelementptr inbounds nuw i8, ptr %5, i64 16
  store i64 0, ptr %7, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %4)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %4, ptr noundef nonnull align 8 dereferenceable(32) %1, i64 32, i1 false)
  %8 = getelementptr inbounds nuw i8, ptr %4, i64 24
  %9 = load ptr, ptr %8, align 8, !alias.scope !6122, !noalias !6127, !nonnull !1740, !noundef !1740
  %10 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %11 = load ptr, ptr %10, align 8, !alias.scope !6122, !noalias !6127
  %12 = icmp eq ptr %11, %9
  br i1 %12, label %.loopexit, label %13

13:                                               ; preds = %2
  %14 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %15 = getelementptr inbounds nuw i8, ptr %3, i64 152
  br label %20

16:                                               ; preds = %27, %18
  %17 = phi { ptr, i32 } [ %19, %18 ], [ %37, %27 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %5) #89
          to label %55 unwind label %53

18:                                               ; preds = %41
  %19 = landingpad { ptr, i32 }
          cleanup
  br label %16

20:                                               ; preds = %48, %13
  %21 = phi ptr [ inttoptr (i64 8 to ptr), %13 ], [ %44, %48 ]
  %22 = phi i64 [ 0, %13 ], [ %46, %48 ]
  %23 = phi ptr [ %11, %13 ], [ %24, %48 ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6130)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6131)
  %24 = getelementptr inbounds nuw i8, ptr %23, i64 200
  %25 = load i64, ptr %23, align 8, !noalias !6132
  %26 = icmp eq i64 %25, -1
  br i1 %26, label %.loopexit, label %28

27:                                               ; preds = %36
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %4)
          to label %16 unwind label %53

28:                                               ; preds = %20
  %29 = getelementptr inbounds nuw i8, ptr %23, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %3)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %14, ptr noundef nonnull align 8 dereferenceable(152) %29, i64 152, i1 false)
  store i64 %25, ptr %3, align 8
  %30 = load i8, ptr %15, align 8, !range !1747, !noundef !1740
  tail call void @llvm.experimental.noalias.scope.decl(metadata !6133)
  %31 = load i64, ptr %5, align 8, !range !1835, !alias.scope !6133, !noalias !6136, !noundef !1740
  %32 = icmp eq i64 %22, %31
  br i1 %32, label %33, label %43

33:                                               ; preds = %28
; invoke <alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %5)
          to label %34 unwind label %36, !noalias !6136

34:                                               ; preds = %33
  %35 = load ptr, ptr %6, align 8, !alias.scope !6133, !noalias !6136
  br label %43

36:                                               ; preds = %33
  %37 = landingpad { ptr, i32 }
          cleanup
  store ptr %24, ptr %10, align 8
; invoke core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_sparql_eval::row_checkpoint::WorkerLedger>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(160) %3) #89
          to label %27 unwind label %38, !noalias !6133

38:                                               ; preds = %36
  %39 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90, !noalias !6138
  unreachable

.loopexit:                                        ; preds = %48, %20, %2
  %40 = phi ptr [ %11, %2 ], [ %24, %20 ], [ %24, %48 ]
  store ptr %40, ptr %10, align 8
  br label %41

41:                                               ; preds = %50, %.loopexit
  %42 = phi i8 [ 1, %50 ], [ 0, %.loopexit ]
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::RowCheckpoint>>(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(32) %4)
          to label %51 unwind label %18

43:                                               ; preds = %34, %28
  %44 = phi ptr [ %35, %34 ], [ %21, %28 ]
  %45 = getelementptr inbounds nuw [160 x i8], ptr %44, i64 %22
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(160) %45, ptr noundef nonnull readonly align 8 dereferenceable(160) %3, i64 160, i1 false), !noalias !6133
  %46 = add i64 %22, 1
  store i64 %46, ptr %7, align 8, !alias.scope !6133, !noalias !6136
  %47 = trunc nuw i8 %30 to i1
  br i1 %47, label %50, label %48

48:                                               ; preds = %43
  call void @llvm.lifetime.end.p0(ptr nonnull %3)
  %49 = icmp eq ptr %24, %9
  br i1 %49, label %.loopexit, label %20

50:                                               ; preds = %43
  store ptr %24, ptr %10, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %3)
  br label %41

51:                                               ; preds = %41
  call void @llvm.lifetime.end.p0(ptr nonnull %4)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %0, ptr noundef nonnull align 8 dereferenceable(24) %5, i64 24, i1 false)
  %52 = getelementptr inbounds nuw i8, ptr %0, i64 24
  store i8 %42, ptr %52, align 8
  call void @llvm.lifetime.end.p0(ptr nonnull %5)
  ret void

53:                                               ; preds = %27, %16
  %54 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #90
  unreachable

55:                                               ; preds = %16
  resume { ptr, i32 } %17
}
define internal fastcc void @purrdf_sparql_eval::row_checkpoint::commit_items::<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, <purrdf_sparql_eval::row_checkpoint::RowCheckpoint>::commit_rows<purrdf_core::ir::dataset::RdfDataset, purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, purrdf_core::small::SmallVec<[purrdf_sparql_eval::row_checkpoint::RowCheckpoint; 1]>>::{closure#0}>(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 16 captures(none) dereferenceable(96) %0, ptr noalias nofree noundef nonnull align 16 dereferenceable(1248) %1, i8 noundef range(i8 0, 17) %2, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(32) %3, ptr noalias nofree noundef nonnull readonly align 8 captures(none) dead_on_return dereferenceable(24) %4, i64 noundef range(i64 0, 2) %5) unnamed_addr #0 personality ptr @rust_eh_personality !guid !15638 {
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
  %29 = alloca [24 x i8], align 8
  %30 = alloca [24 x i8], align 8
  %31 = alloca [24 x i8], align 8
  %32 = alloca [24 x i8], align 8
  %33 = alloca [24 x i8], align 8
  %34 = alloca [24 x i8], align 8
  %35 = alloca [32 x i8], align 8
  %36 = alloca [160 x i8], align 8
  %37 = alloca [32 x i8], align 8
  %38 = alloca [8 x i8], align 8
  %39 = alloca [24 x i8], align 8
  %40 = alloca [32 x i8], align 8
  %41 = alloca [32 x i8], align 8
  %42 = alloca [24 x i8], align 8
  %43 = alloca [96 x i8], align 16
  %44 = alloca [24 x i8], align 8
  %45 = alloca [24 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %45)
  call void @llvm.lifetime.start.p0(ptr nonnull %44)
  call void @llvm.lifetime.start.p0(ptr nonnull %43)
  call void @llvm.lifetime.start.p0(ptr nonnull %42)
  store i64 0, ptr %42, align 8
  %46 = getelementptr inbounds nuw i8, ptr %42, i64 8
  store ptr inttoptr (i64 8 to ptr), ptr %46, align 8
  %47 = getelementptr inbounds nuw i8, ptr %42, i64 16
  store i64 0, ptr %47, align 8
  %48 = getelementptr inbounds nuw i8, ptr %4, i64 16
  %49 = load i64, ptr %48, align 8, !noundef !1740
  %50 = icmp ult i64 %49, 230584300921369396
  tail call void @llvm.assume(i1 %50)
; invoke purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>
  invoke fastcc void @purrdf_sparql_eval::parallel::reserve_concatenated_rows::<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>, [usize; 1]>(ptr noalias nofree noundef align 16 captures(none) dereferenceable(96) %43, ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %42, i64 noundef %49)
          to label %51 unwind label %1295

51:                                               ; preds = %6
  call void @llvm.lifetime.end.p0(ptr nonnull %42)
  %52 = load i64, ptr %43, align 16, !range !2527, !noundef !1740
  %53 = icmp eq i64 %52, -1
  %54 = getelementptr inbounds nuw i8, ptr %43, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %44, ptr noundef nonnull align 8 dereferenceable(24) %54, i64 24, i1 false)
  br i1 %53, label %136, label %55

55:                                               ; preds = %51
  %56 = getelementptr inbounds nuw i8, ptr %43, i64 32
  %57 = getelementptr inbounds nuw i8, ptr %0, i64 32
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 16 dereferenceable(64) %57, ptr noundef nonnull align 16 dereferenceable(64) %56, i64 64, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %43)
  %58 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %58, ptr noundef nonnull align 8 dereferenceable(24) %44, i64 24, i1 false)
  store i64 %52, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %44)
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15639)
  %59 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %60 = load ptr, ptr %59, align 8, !alias.scope !15639, !nonnull !1740, !noundef !1740
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15642)
  %61 = icmp eq i64 %49, 0
  br i1 %61, label %.loopexit168, label %.preheader167

.preheader167:                                    ; preds = %55
  %62 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %63 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %64

64:                                               ; preds = %.preheader167, %102
  %65 = phi i64 [ %67, %102 ], [ 0, %.preheader167 ]
  %66 = getelementptr inbounds nuw [40 x i8], ptr %60, i64 %65
  %67 = add nuw nsw i64 %65, 1
  %68 = load i64, ptr %66, align 8, !range !1778, !alias.scope !15645, !noalias !15639, !noundef !1740
  %69 = icmp ugt i64 %68, 5
  br i1 %69, label %70, label %102

70:                                               ; preds = %64
  %71 = getelementptr i8, ptr %66, i64 8
  %72 = load ptr, ptr %71, align 8, !alias.scope !15642, !noalias !15639, !nonnull !1740, !noundef !1740
  %73 = shl i64 %68, 3
  %74 = add i64 %73, -8
  %75 = load i64, ptr %62, align 8, !noalias !15648, !noundef !1740
  %76 = tail call i64 @llvm.umin.i64(i64 %74, i64 9223372036854775807)
  %77 = tail call i64 @llvm.ssub.sat.i64(i64 %75, i64 %76)
  store i64 %77, ptr %62, align 8, !noalias !15648
  %78 = load i64, ptr %63, align 8, !noalias !15648, !noundef !1740
  %79 = icmp slt i64 %77, %78
  br i1 %79, label %80, label %.preheader516

80:                                               ; preds = %70
  store i64 %77, ptr %63, align 8, !noalias !15648
  br label %.preheader516

.preheader516:                                    ; preds = %80, %70
  br label %81

81:                                               ; preds = %.preheader516, %84
  %82 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15648
  %83 = icmp slt i64 %82, 0
  br i1 %83, label %84, label %__rustc::__rust_dealloc (.exit)

84:                                               ; preds = %81
  %85 = add nsw i64 %82, 1
  %86 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %82, i64 %85 acq_rel acquire, align 8, !noalias !15648
  %87 = extractvalue { i64, i1 } %86, 1
  br i1 %87, label %88, label %81

88:                                               ; preds = %84
  %89 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %76 monotonic, align 8, !noalias !15648
  %90 = tail call i64 @llvm.ssub.sat.i64(i64 %89, i64 %76)
  %91 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15648
  br label %92

92:                                               ; preds = %95, %88
  %93 = phi i64 [ %91, %88 ], [ %98, %95 ]
  %94 = icmp slt i64 %90, %93
  br i1 %94, label %95, label %99

95:                                               ; preds = %92
  %96 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %93, i64 %90 monotonic monotonic, align 8, !noalias !15648
  %97 = extractvalue { i64, i1 } %96, 1
  %98 = extractvalue { i64, i1 } %96, 0
  br i1 %97, label %99, label %92

99:                                               ; preds = %95, %92
  %100 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15648
  br label %__rustc::__rust_dealloc (.exit)

__rustc::__rust_dealloc (.exit): ; preds = %81, %99
  %101 = icmp ne i64 %74, 0
  tail call void @llvm.assume(i1 %101), !noalias !15648
  tail call void @free(ptr noundef nonnull %72) #92, !noalias !15648
  br label %102

102:                                              ; preds = %__rustc::__rust_dealloc (.exit), %64
  %103 = icmp eq i64 %67, %49
  br i1 %103, label %.loopexit168, label %64

.loopexit168:                                     ; preds = %102, %55
  %104 = load i64, ptr %4, align 8, !alias.scope !15639
  %105 = icmp eq i64 %104, 0
  br i1 %105, label %1294, label %106

106:                                              ; preds = %.loopexit168
  %107 = mul nuw i64 %104, 40
  %108 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %109 = load i64, ptr %108, align 8, !noalias !15639, !noundef !1740
  %110 = tail call i64 @llvm.umin.i64(i64 %107, i64 9223372036854775807)
  %111 = tail call i64 @llvm.ssub.sat.i64(i64 %109, i64 %110)
  store i64 %111, ptr %108, align 8, !noalias !15639
  %112 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %113 = load i64, ptr %112, align 8, !noalias !15639, !noundef !1740
  %114 = icmp slt i64 %111, %113
  br i1 %114, label %115, label %.preheader515

115:                                              ; preds = %106
  store i64 %111, ptr %112, align 8, !noalias !15639
  br label %.preheader515

.preheader515:                                    ; preds = %115, %106
  br label %116

116:                                              ; preds = %.preheader515, %119
  %117 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15639
  %118 = icmp slt i64 %117, 0
  br i1 %118, label %119, label %__rustc::__rust_dealloc (.exit118)

119:                                              ; preds = %116
  %120 = add nsw i64 %117, 1
  %121 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %117, i64 %120 acq_rel acquire, align 8, !noalias !15639
  %122 = extractvalue { i64, i1 } %121, 1
  br i1 %122, label %123, label %116

123:                                              ; preds = %119
  %124 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %110 monotonic, align 8, !noalias !15639
  %125 = tail call i64 @llvm.ssub.sat.i64(i64 %124, i64 %110)
  %126 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15639
  br label %127

127:                                              ; preds = %130, %123
  %128 = phi i64 [ %126, %123 ], [ %133, %130 ]
  %129 = icmp slt i64 %125, %128
  br i1 %129, label %130, label %134

130:                                              ; preds = %127
  %131 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %128, i64 %125 monotonic monotonic, align 8, !noalias !15639
  %132 = extractvalue { i64, i1 } %131, 1
  %133 = extractvalue { i64, i1 } %131, 0
  br i1 %132, label %134, label %127

134:                                              ; preds = %130, %127
  %135 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15639
  br label %__rustc::__rust_dealloc (.exit118)

__rustc::__rust_dealloc (.exit118): ; preds = %116, %134
  tail call void @free(ptr noundef nonnull %60) #92, !noalias !15639
  br label %1294

136:                                              ; preds = %51
  call void @llvm.lifetime.end.p0(ptr nonnull %43)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %45, ptr noundef nonnull align 8 dereferenceable(24) %44, i64 24, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %44)
  call void @llvm.lifetime.start.p0(ptr nonnull %41)
  %137 = getelementptr inbounds nuw i8, ptr %4, i64 8
  %138 = load ptr, ptr %137, align 8, !nonnull !1740, !noundef !1740
  %139 = load i64, ptr %4, align 8, !range !1835, !noundef !1740
  %140 = getelementptr inbounds nuw [40 x i8], ptr %138, i64 %49
  store ptr %138, ptr %41, align 8
  %141 = getelementptr inbounds nuw i8, ptr %41, i64 16
  store i64 %139, ptr %141, align 8
  %142 = getelementptr inbounds nuw i8, ptr %41, i64 8
  store ptr %138, ptr %142, align 8
  %143 = getelementptr inbounds nuw i8, ptr %41, i64 24
  store ptr %140, ptr %143, align 8
  %144 = getelementptr inbounds nuw i8, ptr %1, i64 616
  %145 = load ptr, ptr %144, align 8, !noundef !1740
  %146 = icmp eq ptr %145, null
  br i1 %146, label %150, label %147

147:                                              ; preds = %136
  %148 = atomicrmw add ptr %145, i64 1 monotonic, align 8
  %149 = icmp slt i64 %148, 0
  br i1 %149, label %287, label %275

150:                                              ; preds = %136
  call void @llvm.lifetime.start.p0(ptr nonnull %40)
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(32) %40, ptr noundef nonnull align 8 dereferenceable(32) %41, i64 32, i1 false)
  %151 = getelementptr inbounds nuw i8, ptr %40, i64 24
  %152 = load ptr, ptr %151, align 8, !alias.scope !15651, !noalias !15654, !nonnull !1740, !noundef !1740
  %153 = getelementptr inbounds nuw i8, ptr %40, i64 8
  %154 = load ptr, ptr %153, align 8, !alias.scope !15651, !noalias !15654
  %155 = icmp eq ptr %154, %152
  br i1 %155, label %.loopexit137, label %156

156:                                              ; preds = %150
  %157 = getelementptr inbounds nuw i8, ptr %45, i64 16
  %158 = getelementptr inbounds nuw i8, ptr %45, i64 8
  br label %160

159:                                              ; preds = %263, %260
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %40) #89
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %45) #89
  br label %1299

160:                                              ; preds = %266, %156
  %161 = phi ptr [ %154, %156 ], [ %162, %266 ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15651)
  %162 = getelementptr inbounds nuw i8, ptr %161, i64 40
  %163 = load i64, ptr %161, align 8, !noalias !15651
  %164 = getelementptr inbounds nuw i8, ptr %161, i64 8
  %165 = load ptr, ptr %164, align 8, !noalias !15651
  %166 = icmp eq i64 %163, 0
  br i1 %166, label %.loopexit137, label %254

.loopexit137:                                     ; preds = %266, %160, %150
  %167 = phi ptr [ %154, %150 ], [ %162, %160 ], [ %162, %266 ]
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15656)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15659)
  %168 = ptrtoint ptr %152 to i64
  %169 = ptrtoint ptr %167 to i64
  %170 = sub nuw i64 %168, %169
  %171 = udiv exact i64 %170, 40
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15662)
  %172 = icmp eq ptr %152, %167
  br i1 %172, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %.loopexit137
  %173 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %174 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %175

175:                                              ; preds = %.preheader, %213
  %176 = phi i64 [ %178, %213 ], [ 0, %.preheader ]
  %177 = getelementptr inbounds nuw [40 x i8], ptr %167, i64 %176
  %178 = add nuw nsw i64 %176, 1
  %179 = load i64, ptr %177, align 8, !range !1778, !alias.scope !15665, !noalias !15668, !noundef !1740
  %180 = icmp ugt i64 %179, 5
  br i1 %180, label %181, label %213

181:                                              ; preds = %175
  %182 = getelementptr i8, ptr %177, i64 8
  %183 = load ptr, ptr %182, align 8, !alias.scope !15662, !noalias !15668, !nonnull !1740, !noundef !1740
  %184 = shl i64 %179, 3
  %185 = add i64 %184, -8
  %186 = load i64, ptr %173, align 8, !noalias !15669, !noundef !1740
  %187 = tail call i64 @llvm.umin.i64(i64 %185, i64 9223372036854775807)
  %188 = tail call i64 @llvm.ssub.sat.i64(i64 %186, i64 %187)
  store i64 %188, ptr %173, align 8, !noalias !15669
  %189 = load i64, ptr %174, align 8, !noalias !15669, !noundef !1740
  %190 = icmp slt i64 %188, %189
  br i1 %190, label %191, label %.preheader267

191:                                              ; preds = %181
  store i64 %188, ptr %174, align 8, !noalias !15669
  br label %.preheader267

.preheader267:                                    ; preds = %191, %181
  br label %192

192:                                              ; preds = %.preheader267, %195
  %193 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15669
  %194 = icmp slt i64 %193, 0
  br i1 %194, label %195, label %__rustc::__rust_dealloc (.exit119)

195:                                              ; preds = %192
  %196 = add nsw i64 %193, 1
  %197 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %193, i64 %196 acq_rel acquire, align 8, !noalias !15669
  %198 = extractvalue { i64, i1 } %197, 1
  br i1 %198, label %199, label %192

199:                                              ; preds = %195
  %200 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %187 monotonic, align 8, !noalias !15669
  %201 = tail call i64 @llvm.ssub.sat.i64(i64 %200, i64 %187)
  %202 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15669
  br label %203

203:                                              ; preds = %206, %199
  %204 = phi i64 [ %202, %199 ], [ %209, %206 ]
  %205 = icmp slt i64 %201, %204
  br i1 %205, label %206, label %210

206:                                              ; preds = %203
  %207 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %204, i64 %201 monotonic monotonic, align 8, !noalias !15669
  %208 = extractvalue { i64, i1 } %207, 1
  %209 = extractvalue { i64, i1 } %207, 0
  br i1 %208, label %210, label %203

210:                                              ; preds = %206, %203
  %211 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15669
  br label %__rustc::__rust_dealloc (.exit119)

__rustc::__rust_dealloc (.exit119): ; preds = %192, %210
  %212 = icmp ne i64 %185, 0
  tail call void @llvm.assume(i1 %212), !noalias !15669
  tail call void @free(ptr noundef nonnull %183) #92, !noalias !15669
  br label %213

213:                                              ; preds = %__rustc::__rust_dealloc (.exit119), %175
  %214 = icmp eq i64 %178, %171
  br i1 %214, label %.loopexit, label %175

.loopexit:                                        ; preds = %213, %.loopexit137
  %215 = getelementptr inbounds nuw i8, ptr %40, i64 16
  %216 = load i64, ptr %215, align 8, !alias.scope !15668, !noundef !1740
  %217 = icmp eq i64 %216, 0
  br i1 %217, label %251, label %218

218:                                              ; preds = %.loopexit
  %219 = load ptr, ptr %40, align 8, !alias.scope !15668, !nonnull !1740, !noundef !1740
  %220 = mul nuw i64 %216, 40
  %221 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %222 = load i64, ptr %221, align 8, !noalias !15668, !noundef !1740
  %223 = tail call i64 @llvm.umin.i64(i64 %220, i64 9223372036854775807)
  %224 = tail call i64 @llvm.ssub.sat.i64(i64 %222, i64 %223)
  store i64 %224, ptr %221, align 8, !noalias !15668
  %225 = tail call noundef nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %226 = load i64, ptr %225, align 8, !noalias !15668, !noundef !1740
  %227 = icmp slt i64 %224, %226
  br i1 %227, label %228, label %.preheader266

228:                                              ; preds = %218
  store i64 %224, ptr %225, align 8, !noalias !15668
  br label %.preheader266

.preheader266:                                    ; preds = %228, %218
  br label %229

229:                                              ; preds = %.preheader266, %232
  %230 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15668
  %231 = icmp slt i64 %230, 0
  br i1 %231, label %232, label %__rustc::__rust_dealloc (.exit120)

232:                                              ; preds = %229
  %233 = add nsw i64 %230, 1
  %234 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %230, i64 %233 acq_rel acquire, align 8, !noalias !15668
  %235 = extractvalue { i64, i1 } %234, 1
  br i1 %235, label %236, label %229

236:                                              ; preds = %232
  %237 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %223 monotonic, align 8, !noalias !15668
  %238 = tail call i64 @llvm.ssub.sat.i64(i64 %237, i64 %223)
  %239 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15668
  br label %240

240:                                              ; preds = %243, %236
  %241 = phi i64 [ %239, %236 ], [ %246, %243 ]
  %242 = icmp slt i64 %238, %241
  br i1 %242, label %243, label %247

243:                                              ; preds = %240
  %244 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %241, i64 %238 monotonic monotonic, align 8, !noalias !15668
  %245 = extractvalue { i64, i1 } %244, 1
  %246 = extractvalue { i64, i1 } %244, 0
  br i1 %245, label %247, label %240

247:                                              ; preds = %243, %240
  %248 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15668
  br label %__rustc::__rust_dealloc (.exit120)

__rustc::__rust_dealloc (.exit120): ; preds = %229, %247
  tail call void @free(ptr noundef nonnull %219) #92, !noalias !15668
  br label %251

249:                                              ; preds = %1204, %435
  %250 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %41) #89
  br label %1297

251:                                              ; preds = %__rustc::__rust_dealloc (.exit120), %.loopexit
  call void @llvm.lifetime.end.p0(ptr nonnull %40)
  %252 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %252, ptr noundef nonnull align 8 dereferenceable(24) %45, i64 24, i1 false)
  %253 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 2, ptr %253, align 16
  store i64 -1, ptr %0, align 16
  call void @llvm.lifetime.end.p0(ptr nonnull %41)
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  br label %1294

254:                                              ; preds = %160
  %255 = getelementptr inbounds nuw i8, ptr %161, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %39, ptr noundef nonnull align 8 dereferenceable(24) %255, i64 24, i1 false)
  tail call void @llvm.experimental.noalias.scope.decl(metadata !15672)
  %256 = load i64, ptr %157, align 8, !alias.scope !15672, !noalias !15675, !noundef !1740
  %257 = load i64, ptr %45, align 8, !range !1835, !alias.scope !15672, !noalias !15675, !noundef !1740
  %258 = icmp eq i64 %256, %257
  br i1 %258, label %259, label %266

259:                                              ; preds = %254
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %45)
          to label %266 unwind label %260, !noalias !15675

260:                                              ; preds = %259
  %261 = landingpad { ptr, i32 }
          cleanup
  store ptr %162, ptr %153, align 8
  %262 = icmp ugt i64 %163, 5
  br i1 %262, label %263, label %159

263:                                              ; preds = %260
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %165) ]
  %264 = shl i64 %163, 3
  %265 = add i64 %264, -8
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %165, i64 noundef %265, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !15677
  br label %159

266:                                              ; preds = %259, %254
  %267 = load ptr, ptr %158, align 8, !alias.scope !15672, !noalias !15675, !nonnull !1740, !noundef !1740
  %268 = getelementptr inbounds nuw [40 x i8], ptr %267, i64 %256
  store i64 %163, ptr %268, align 8, !noalias !15672
  %269 = getelementptr inbounds nuw i8, ptr %268, i64 8
  store ptr %165, ptr %269, align 8, !noalias !15672
  %270 = getelementptr inbounds nuw i8, ptr %268, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %270, ptr noundef nonnull align 8 dereferenceable(24) %39, i64 24, i1 false), !noalias !15672
  %271 = add i64 %256, 1
  store i64 %271, ptr %157, align 8, !alias.scope !15672, !noalias !15675
  %272 = icmp eq ptr %162, %152
  br i1 %272, label %.loopexit137, label %160

273:                                              ; preds = %1299, %532, %396, %381
  %274 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  call void @core::panicking::panic_in_cleanup() #90
  unreachable

275:                                              ; preds = %147
  %276 = load ptr, ptr %144, align 8, !nonnull !1740, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %38)
  store ptr %276, ptr %38, align 8
  %277 = getelementptr inbounds nuw i8, ptr %276, i64 16
  %278 = load i64, ptr %277, align 8
  %279 = icmp eq i64 %278, -1
  %280 = getelementptr inbounds nuw i8, ptr %276, i64 40
  %281 = load i64, ptr %280, align 8
  %282 = icmp ne i64 %281, -1
  %283 = getelementptr inbounds nuw i8, ptr %3, i64 8
  %284 = load ptr, ptr %283, align 8
  %285 = getelementptr inbounds nuw i8, ptr %3, i64 16
  %286 = load i64, ptr %285, align 8
  br i1 %282, label %354, label %288

287:                                              ; preds = %147
  tail call void @llvm.trap()
  unreachable

288:                                              ; preds = %393, %389, %354, %275
  %289 = phi i64 [ %286, %389 ], [ 0, %354 ], [ %286, %275 ], [ %286, %393 ]
  %290 = load i64, ptr %3, align 8, !range !1835, !noundef !1740
  %291 = icmp ult i64 %289, 57646075230342349
  tail call void @llvm.assume(i1 %291)
  %292 = mul nuw nsw i64 %289, 160
  %293 = getelementptr inbounds nuw i8, ptr %284, i64 %292
  call void @llvm.lifetime.start.p0(ptr nonnull %37)
  store ptr %284, ptr %37, align 8
  %294 = getelementptr inbounds nuw i8, ptr %37, i64 8
  store ptr %284, ptr %294, align 8
  %295 = getelementptr inbounds nuw i8, ptr %37, i64 16
  store i64 %290, ptr %295, align 8
  %296 = getelementptr inbounds nuw i8, ptr %37, i64 24
  store ptr %293, ptr %296, align 8
  %297 = icmp eq i64 %289, 0
  br i1 %297, label %.loopexit165, label %298

298:                                              ; preds = %288
  %299 = getelementptr inbounds nuw i8, ptr %36, i64 8
  %300 = getelementptr inbounds nuw i8, ptr %36, i64 24
  %301 = getelementptr inbounds nuw i8, ptr %36, i64 32
  %302 = getelementptr inbounds nuw i8, ptr %36, i64 40
  %303 = getelementptr inbounds nuw i8, ptr %36, i64 16
  %304 = getelementptr inbounds nuw i8, ptr %35, i64 16
  %305 = getelementptr inbounds nuw i8, ptr %35, i64 8
  %306 = getelementptr inbounds nuw i8, ptr %35, i64 24
  %307 = getelementptr inbounds nuw i8, ptr %36, i64 96
  %308 = getelementptr inbounds nuw i8, ptr %36, i64 48
  %309 = getelementptr inbounds nuw i8, ptr %36, i64 56
  %310 = getelementptr inbounds nuw i8, ptr %36, i64 64
  %311 = getelementptr inbounds nuw i8, ptr %276, i64 80
  %312 = getelementptr inbounds nuw i8, ptr %23, i64 1
  %313 = getelementptr inbounds nuw i8, ptr %23, i64 8
  %314 = getelementptr inbounds nuw i8, ptr %23, i64 16
  %315 = getelementptr inbounds nuw i8, ptr %1, i64 632
  %316 = getelementptr inbounds nuw i8, ptr %1, i64 1228
  %317 = zext nneg i8 %2 to i64
  %318 = getelementptr inbounds nuw i8, ptr %276, i64 296
  %319 = getelementptr inbounds nuw i8, ptr %276, i64 272
  %320 = getelementptr inbounds nuw i8, ptr %1, i64 1048
  %321 = getelementptr inbounds nuw i8, ptr %1, i64 1056
  %322 = getelementptr inbounds nuw i8, ptr %276, i64 104
  %323 = getelementptr inbounds nuw i8, ptr %17, i64 1
  %324 = getelementptr inbounds nuw i8, ptr %17, i64 8
  %325 = getelementptr inbounds nuw i8, ptr %17, i64 16
  %326 = getelementptr inbounds nuw i8, ptr %22, i64 8
  %327 = getelementptr inbounds nuw i8, ptr %1, i64 888
  %328 = getelementptr inbounds nuw i8, ptr %11, i64 1
  %329 = getelementptr inbounds nuw i8, ptr %11, i64 8
  %330 = getelementptr inbounds nuw i8, ptr %11, i64 16
  %331 = getelementptr inbounds nuw i8, ptr %18, i64 8
  %332 = getelementptr inbounds nuw i8, ptr %20, i64 1
  %333 = getelementptr inbounds nuw i8, ptr %20, i64 8
  %334 = getelementptr inbounds nuw i8, ptr %20, i64 16
  %335 = getelementptr inbounds nuw i8, ptr %19, i64 8
  %336 = getelementptr inbounds nuw i8, ptr %16, i64 8
  %337 = getelementptr inbounds nuw i8, ptr %15, i64 8
  %338 = getelementptr inbounds nuw i8, ptr %13, i64 1
  %339 = getelementptr inbounds nuw i8, ptr %13, i64 8
  %340 = getelementptr inbounds nuw i8, ptr %13, i64 16
  %341 = getelementptr inbounds nuw i8, ptr %12, i64 8
  %342 = getelementptr inbounds nuw i8, ptr %45, i64 16
  %343 = getelementptr inbounds nuw i8, ptr %45, i64 8
  %344 = getelementptr inbounds nuw i8, ptr %36, i64 88
  %345 = getelementptr inbounds nuw i8, ptr %36, i64 120
  %346 = getelementptr inbounds nuw i8, ptr %8, i64 1
  %347 = getelementptr inbounds nuw i8, ptr %8, i64 8
  %348 = getelementptr inbounds nuw i8, ptr %8, i64 16
  %349 = getelementptr inbounds nuw i8, ptr %7, i64 1
  %350 = getelementptr inbounds nuw i8, ptr %7, i64 8
  %351 = getelementptr inbounds nuw i8, ptr %7, i64 16
  %352 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_LIVE_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  %353 = tail call nonnull align 8 ptr @llvm.threadlocal.address.p0(ptr @purrdf_alloc_probe::THREAD_TROUGH_BYTES::{K#0}::{closure#1}::__RUST_STD_INTERNAL_VAL (.llvm.6551227014246703601))
  br label %397

354:                                              ; preds = %275
  %355 = icmp eq i64 %286, 0
  br i1 %355, label %288, label %iter.check

iter.check:                                       ; preds = %354
  %min.iters.check = icmp ult i64 %286, 8
  br i1 %min.iters.check, label %.preheader166.preheader, label %vector.main.loop.iter.check

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check237 = icmp ult i64 %286, 32
  br i1 %min.iters.check237, label %vec.epilog.ph, label %vector.ph

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.mod.vf = and i64 %286, 24
  %n.vec = and i64 %286, -32
  br label %vector.body

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ]
  %vec.ind = phi <8 x i64> [ <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>, %vector.ph ], [ %vec.ind.next, %vector.body ]
  %vec.phi = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %356, %vector.body ]
  %vec.phi238 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %357, %vector.body ]
  %vec.phi239 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %358, %vector.body ]
  %vec.phi240 = phi <8 x i64> [ zeroinitializer, %vector.ph ], [ %359, %vector.body ]
  %step.add = add nuw <8 x i64> %vec.ind, splat (i64 8)
  %step.add.2 = add nuw <8 x i64> %vec.ind, splat (i64 16)
  %step.add.3 = add nuw <8 x i64> %vec.ind, splat (i64 24)
  %wide.gep = getelementptr inbounds nuw [160 x i8], ptr %284, <8 x i64> %vec.ind
  %wide.gep241 = getelementptr inbounds nuw [160 x i8], ptr %284, <8 x i64> %step.add
  %wide.gep242 = getelementptr inbounds nuw [160 x i8], ptr %284, <8 x i64> %step.add.2
  %wide.gep243 = getelementptr inbounds nuw [160 x i8], ptr %284, <8 x i64> %step.add.3
  %wide.gep244 = getelementptr i8, <8 x ptr> %wide.gep, i64 16
  %wide.gep245 = getelementptr i8, <8 x ptr> %wide.gep241, i64 16
  %wide.gep246 = getelementptr i8, <8 x ptr> %wide.gep242, i64 16
  %wide.gep247 = getelementptr i8, <8 x ptr> %wide.gep243, i64 16
  %wide.masked.gather = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep244, <8 x i1> splat (i1 true), <8 x i64> poison)
  %wide.masked.gather248 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep245, <8 x i1> splat (i1 true), <8 x i64> poison)
  %wide.masked.gather249 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep246, <8 x i1> splat (i1 true), <8 x i64> poison)
  %wide.masked.gather250 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep247, <8 x i1> splat (i1 true), <8 x i64> poison)
  %356 = add <8 x i64> %wide.masked.gather, %vec.phi
  %357 = add <8 x i64> %wide.masked.gather248, %vec.phi238
  %358 = add <8 x i64> %wide.masked.gather249, %vec.phi239
  %359 = add <8 x i64> %wide.masked.gather250, %vec.phi240
  %index.next = add nuw i64 %index, 32
  %vec.ind.next = add nuw <8 x i64> %vec.ind, splat (i64 32)
  %360 = icmp eq i64 %index.next, %n.vec
  br i1 %360, label %middle.block, label %vector.body, !llvm.loop !15680

middle.block:                                     ; preds = %vector.body
  %bin.rdx = add <8 x i64> %357, %356
  %bin.rdx251 = add <8 x i64> %358, %bin.rdx
  %bin.rdx252 = add <8 x i64> %359, %bin.rdx251
  %361 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %bin.rdx252)
  %cmp.n = icmp eq i64 %286, %n.vec
  br i1 %cmp.n, label %.loopexit265, label %vec.epilog.iter.check

vec.epilog.iter.check:                            ; preds = %middle.block
  %min.epilog.iters.check = icmp eq i64 %n.mod.vf, 0
  br i1 %min.epilog.iters.check, label %.preheader166.preheader, label %vec.epilog.ph, !prof !11074

vec.epilog.ph:                                    ; preds = %vector.main.loop.iter.check, %vec.epilog.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %bc.merge.rdx = phi i64 [ %361, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec254 = and i64 %286, -8
  %362 = insertelement <8 x i64> <i64 poison, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0>, i64 %bc.merge.rdx, i64 0
  %broadcast.splatinsert = insertelement <8 x i64> poison, i64 %vec.epilog.resume.val, i64 0
  %broadcast.splat = shufflevector <8 x i64> %broadcast.splatinsert, <8 x i64> poison, <8 x i32> zeroinitializer
  %induction = or disjoint <8 x i64> %broadcast.splat, <i64 0, i64 1, i64 2, i64 3, i64 4, i64 5, i64 6, i64 7>
  br label %vec.epilog.vector.body

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index255 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next261, %vec.epilog.vector.body ]
  %vec.ind256 = phi <8 x i64> [ %induction, %vec.epilog.ph ], [ %vec.ind.next262, %vec.epilog.vector.body ]
  %vec.phi257 = phi <8 x i64> [ %362, %vec.epilog.ph ], [ %363, %vec.epilog.vector.body ]
  %wide.gep258 = getelementptr inbounds nuw [160 x i8], ptr %284, <8 x i64> %vec.ind256
  %wide.gep259 = getelementptr i8, <8 x ptr> %wide.gep258, i64 16
  %wide.masked.gather260 = tail call <8 x i64> @llvm.masked.gather.v8i64.v8p0(<8 x ptr> align 8 %wide.gep259, <8 x i1> splat (i1 true), <8 x i64> poison)
  %363 = add <8 x i64> %wide.masked.gather260, %vec.phi257
  %index.next261 = add nuw i64 %index255, 8
  %vec.ind.next262 = add nuw <8 x i64> %vec.ind256, splat (i64 8)
  %364 = icmp eq i64 %index.next261, %n.vec254
  br i1 %364, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !llvm.loop !15681

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %365 = tail call i64 @llvm.vector.reduce.add.v8i64(<8 x i64> %363)
  %cmp.n263 = icmp eq i64 %286, %n.vec254
  br i1 %cmp.n263, label %.loopexit265, label %.preheader166.preheader

.preheader166.preheader:                          ; preds = %iter.check, %vec.epilog.iter.check, %vec.epilog.middle.block
  %.ph507 = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec254, %vec.epilog.middle.block ]
  %.ph508 = phi i64 [ 0, %iter.check ], [ %361, %vec.epilog.iter.check ], [ %365, %vec.epilog.middle.block ]
  br label %.preheader166

.preheader166:                                    ; preds = %.preheader166.preheader, %.preheader166
  %366 = phi i64 [ %373, %.preheader166 ], [ %.ph507, %.preheader166.preheader ]
  %367 = phi i64 [ %372, %.preheader166 ], [ %.ph508, %.preheader166.preheader ]
  %368 = getelementptr inbounds nuw [160 x i8], ptr %284, i64 %366
  %369 = getelementptr i8, ptr %368, i64 16
  %370 = load i64, ptr %369, align 8, !noundef !1740
  %371 = icmp ult i64 %370, 104811045873349726
  tail call void @llvm.assume(i1 %371)
  %372 = add i64 %370, %367
  %373 = add nuw i64 %366, 1
  %374 = icmp eq i64 %373, %286
  br i1 %374, label %.loopexit265, label %.preheader166, !llvm.loop !15682

375:                                              ; preds = %.loopexit138, %.loopexit.split-lp, %396
  %376 = phi i1 [ %535, %396 ], [ true, %.loopexit138 ], [ %.ph, %.loopexit.split-lp ]
  %377 = phi i1 [ false, %396 ], [ false, %.loopexit138 ], [ %.ph139, %.loopexit.split-lp ]
  %378 = phi { ptr, i32 } [ %536, %396 ], [ %lpad.loopexit, %.loopexit138 ], [ %lpad.loopexit.split-lp, %.loopexit.split-lp ]
  %379 = atomicrmw sub ptr %276, i64 1 release, align 8, !noalias !15683
  %380 = icmp eq i64 %379, 1
  br i1 %380, label %381, label %1291

381:                                              ; preds = %375
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %38) #91
          to label %1291 unwind label %273

.loopexit138:                                     ; preds = %452
  %lpad.loopexit = landingpad { ptr, i32 }
          cleanup
  br label %375

.loopexit.split-lp:                               ; preds = %388, %393, %.loopexit165, %1200
  %.ph = phi i1 [ true, %393 ], [ true, %.loopexit165 ], [ true, %388 ], [ false, %1200 ]
  %.ph139 = phi i1 [ true, %393 ], [ false, %.loopexit165 ], [ true, %388 ], [ false, %1200 ]
  %lpad.loopexit.split-lp = landingpad { ptr, i32 }
          cleanup
  br label %375

.loopexit265:                                     ; preds = %.preheader166, %vec.epilog.middle.block, %middle.block
  %.lcssa236 = phi i64 [ %365, %vec.epilog.middle.block ], [ %361, %middle.block ], [ %372, %.preheader166 ]
  %382 = getelementptr inbounds nuw i8, ptr %1, i64 912
  %383 = getelementptr inbounds nuw i8, ptr %1, i64 928
  %384 = load i64, ptr %383, align 16, !alias.scope !15688, !noundef !1740
  %385 = load i64, ptr %382, align 16, !range !1835, !alias.scope !15688, !noundef !1740
  %386 = sub i64 %385, %384
  %387 = icmp ugt i64 %.lcssa236, %386
  br i1 %387, label %388, label %389, !prof !15693

388:                                              ; preds = %.loopexit265
; invoke <alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>
  invoke fastcc void @<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global> (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %382, i64 noundef %384, i64 noundef %.lcssa236, i64 noundef 8, i64 noundef 80)
          to label %389 unwind label %.loopexit.split-lp

389:                                              ; preds = %388, %.loopexit265
  %390 = getelementptr inbounds nuw i8, ptr %1, i64 1016
  %391 = load i64, ptr %390, align 8, !alias.scope !15694, !noundef !1740
  %392 = icmp ugt i64 %.lcssa236, %391
  br i1 %392, label %393, label %288, !prof !15693

393:                                              ; preds = %389
  %394 = getelementptr inbounds nuw i8, ptr %1, i64 1000
; invoke <hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>
  %395 = invoke { i64, i64 } @<hashbrown::raw::RawTable<usize>>::reserve_rehash::<<purrdf_sparql_eval::scratch::ScratchInterner>::reserve_ghosts::{closure#0}>(ptr noalias nofree noundef nonnull align 8 dereferenceable(32) %394, i64 noundef %.lcssa236, ptr noundef nonnull align 8 %382, i1 noundef zeroext true) #91
          to label %288 unwind label %.loopexit.split-lp

396:                                              ; preds = %1290, %1287, %1284
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37) #89
          to label %375 unwind label %273

397:                                              ; preds = %595, %298
  %398 = phi ptr [ %284, %298 ], [ %399, %595 ]
  call void @llvm.experimental.noalias.scope.decl(metadata !15697)
  %399 = getelementptr inbounds nuw i8, ptr %398, i64 160
  store ptr %399, ptr %294, align 8, !alias.scope !15697, !noalias !15700
  %400 = load i64, ptr %398, align 8, !noalias !15697
  %401 = icmp eq i64 %400, -1
  br i1 %401, label %.loopexit165, label %402

402:                                              ; preds = %397
  %403 = getelementptr inbounds nuw i8, ptr %398, i64 8
  call void @llvm.lifetime.start.p0(ptr nonnull %36)
  store i64 %400, ptr %36, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(152) %299, ptr noundef nonnull align 8 dereferenceable(152) %403, i64 152, i1 false)
  %404 = load i64, ptr %300, align 8
  %405 = load ptr, ptr %301, align 8
  %406 = load i64, ptr %302, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %35)
  %407 = load ptr, ptr %299, align 8, !nonnull !1740, !noundef !1740
  %408 = load i64, ptr %303, align 8, !noundef !1740
  %409 = icmp ult i64 %408, 104811045873349726
  call void @llvm.assume(i1 %409)
  %410 = getelementptr inbounds nuw [88 x i8], ptr %407, i64 %408
  store ptr %407, ptr %35, align 8
  store i64 %400, ptr %304, align 8
  store ptr %407, ptr %305, align 8
  store ptr %410, ptr %306, align 8
  %411 = load ptr, ptr %309, align 8, !nonnull !1740, !noundef !1740
  %412 = load i64, ptr %308, align 8, !range !1835, !noundef !1740
  %413 = load i64, ptr %310, align 8, !noundef !1740
  %414 = icmp ult i64 %413, 288230376151711744
  call void @llvm.assume(i1 %414)
  %415 = shl nuw nsw i64 %413, 5
  %416 = getelementptr inbounds nuw i8, ptr %411, i64 %415
  %417 = icmp eq i64 %413, 0
  br i1 %417, label %.loopexit164, label %418

418:                                              ; preds = %402
  %419 = load i64, ptr %307, align 8, !noundef !1740
  %420 = icmp ult i64 %406, 384307168202282326
  %421 = ptrtoint ptr %410 to i64
  %422 = load ptr, ptr %142, align 8
  br label %486

.loopexit165:                                     ; preds = %595, %397, %288
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37)
          to label %423 unwind label %.loopexit.split-lp

423:                                              ; preds = %.loopexit165
  call void @llvm.lifetime.end.p0(ptr nonnull %37)
  %424 = getelementptr inbounds nuw i8, ptr %3, i64 24
  %425 = load i8, ptr %424, align 8, !range !1747, !noundef !1740
  %426 = trunc nuw i8 %425 to i1
  %427 = trunc nuw i64 %5 to i1
  %428 = xor i1 %427, true
  %429 = or i1 %428, %426
  %430 = select i1 %429, i1 true, i1 %279
  br i1 %430, label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit), label %436

<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split): ; preds = %.noexc, %448
  call void @llvm.lifetime.end.p0(ptr nonnull %10), !noalias !15702
  br label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)

<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit): ; preds = %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split), %423
  %431 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %431, ptr noundef nonnull align 8 dereferenceable(24) %45, i64 24, i1 false)
  %432 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 2, ptr %432, align 16
  store i64 -1, ptr %0, align 16
  %433 = atomicrmw sub ptr %276, i64 1 release, align 8, !noalias !15709
  %434 = icmp eq i64 %433, 1
  br i1 %434, label %435, label %453

435:                                              ; preds = %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %38) #91
          to label %453 unwind label %249

436:                                              ; preds = %423
  %437 = getelementptr inbounds nuw i8, ptr %276, i64 80
  %438 = getelementptr inbounds nuw i8, ptr %9, i64 1
  %439 = getelementptr inbounds nuw i8, ptr %9, i64 8
  %440 = getelementptr inbounds nuw i8, ptr %9, i64 16
  call void @llvm.lifetime.start.p0(ptr nonnull %10), !noalias !15702
  %441 = load atomic i64, ptr %437 monotonic, align 8, !noalias !15714
  br label %442

442:                                              ; preds = %442, %436
  %443 = phi i64 [ %441, %436 ], [ %447, %442 ]
  %444 = call i64 @llvm.uadd.sat.i64(i64 %443, i64 1)
  %445 = cmpxchg weak ptr %437, i64 %443, i64 %444 monotonic monotonic, align 8, !noalias !15714
  %446 = extractvalue { i64, i1 } %445, 1
  %447 = extractvalue { i64, i1 } %445, 0
  br i1 %446, label %448, label %442

448:                                              ; preds = %442
  %449 = call i64 @llvm.uadd.sat.i64(i64 %447, i64 1)
  %450 = load i64, ptr %277, align 8, !noalias !15714
  %451 = icmp ugt i64 %449, %450
  br i1 %451, label %452, label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split)

452:                                              ; preds = %448
  call void @llvm.lifetime.start.p0(ptr nonnull %9), !noalias !15714
  store i8 0, ptr %438, align 1, !noalias !15714
  store i64 %450, ptr %439, align 8, !noalias !15714
  store i64 %449, ptr %440, align 8, !noalias !15714
  store i8 0, ptr %9, align 8, !noalias !15714
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %10, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %9)
          to label %.noexc unwind label %.loopexit138

.noexc:                                           ; preds = %452
  call void @llvm.lifetime.end.p0(ptr nonnull %9), !noalias !15714
  br label %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit.sink.split)

453:                                              ; preds = %435, %<purrdf_sparql_eval::governor::GovernorState>::commit_reported_items (.exit)
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %41)
  call void @llvm.lifetime.end.p0(ptr nonnull %41)
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  br label %454

454:                                              ; preds = %1294, %1213, %453
  ret void

455:                                              ; preds = %800
  %456 = landingpad { ptr, i32 }
          cleanup
  store ptr %796, ptr %305, align 8
  br label %481

457:                                              ; preds = %778
  %458 = landingpad { ptr, i32 }
          cleanup
  br label %481

459:                                              ; preds = %864
  %460 = landingpad { ptr, i32 }
          cleanup
  store ptr %860, ptr %305, align 8
  br label %481

461:                                              ; preds = %760
  %462 = landingpad { ptr, i32 }
          cleanup
  store ptr %756, ptr %305, align 8
  br label %481

463:                                              ; preds = %1086
  %464 = landingpad { ptr, i32 }
          cleanup
  store ptr %1082, ptr %305, align 8
  br label %481

465:                                              ; preds = %1017
  %466 = landingpad { ptr, i32 }
          cleanup
  br label %481

467:                                              ; preds = %990
  %468 = landingpad { ptr, i32 }
          cleanup
  store ptr %986, ptr %305, align 8
  br label %481

469:                                              ; preds = %932
  %470 = landingpad { ptr, i32 }
          cleanup
  store ptr %928, ptr %305, align 8
  br label %481

471:                                              ; preds = %896, %880, %848
  %472 = landingpad { ptr, i32 }
          cleanup
  br label %481

473:                                              ; preds = %.preheader160
  %474 = landingpad { ptr, i32 }
          cleanup
  br label %481

475:                                              ; preds = %616
  %476 = landingpad { ptr, i32 }
          cleanup
  br label %481

477:                                              ; preds = %1066, %1048, %1053
  %478 = landingpad { ptr, i32 }
          cleanup
  br label %481

479:                                              ; preds = %641, %599
  %480 = landingpad { ptr, i32 }
          cleanup
  br label %481

481:                                              ; preds = %1113, %1110, %479, %477, %475, %473, %471, %469, %467, %465, %463, %461, %459, %457, %455
  %482 = phi { ptr, i32 } [ %1111, %1110 ], [ %1111, %1113 ], [ %456, %455 ], [ %458, %457 ], [ %460, %459 ], [ %462, %461 ], [ %464, %463 ], [ %466, %465 ], [ %468, %467 ], [ %470, %469 ], [ %472, %471 ], [ %474, %473 ], [ %476, %475 ], [ %478, %477 ], [ %480, %479 ]
  %483 = icmp eq i64 %412, 0
  br i1 %483, label %532, label %484

484:                                              ; preds = %481
  %485 = shl nuw i64 %412, 5
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %411, i64 noundef %485, i64 noundef range(i64 1, -9223372036854775807) 8) #92, !noalias !15717
  br label %532

486:                                              ; preds = %.loopexit146, %418
  %487 = phi ptr [ %422, %418 ], [ %1096, %.loopexit146 ]
  %488 = phi ptr [ %407, %418 ], [ %912, %.loopexit146 ]
  %489 = phi i64 [ 0, %418 ], [ %639, %.loopexit146 ]
  %490 = phi ptr [ %411, %418 ], [ %493, %.loopexit146 ]
  %491 = phi ptr [ %407, %418 ], [ %914, %.loopexit146 ]
  %492 = phi i64 [ %419, %418 ], [ %913, %.loopexit146 ]
  %493 = getelementptr inbounds nuw i8, ptr %490, i64 32
  %494 = load i64, ptr %490, align 8, !noalias !15720
  %495 = getelementptr inbounds nuw i8, ptr %490, i64 8
  %496 = load i64, ptr %495, align 8, !noalias !15720
  %497 = getelementptr inbounds nuw i8, ptr %490, i64 16
  %498 = load i64, ptr %497, align 8, !noalias !15720
  %499 = getelementptr inbounds nuw i8, ptr %490, i64 24
  %500 = load i64, ptr %499, align 8, !noalias !15720
  %501 = icmp eq i64 %494, 0
  %502 = select i1 %501, i1 true, i1 %279
  br i1 %502, label %597, label %604

.loopexit164:                                     ; preds = %.loopexit146, %402
  %503 = icmp eq i64 %412, 0
  br i1 %503, label %533, label %504

504:                                              ; preds = %.loopexit164
  %505 = shl nuw i64 %412, 5
  %506 = load i64, ptr %352, align 8, !noalias !15723, !noundef !1740
  %507 = call i64 @llvm.umin.i64(i64 %505, i64 9223372036854775807)
  %508 = call i64 @llvm.ssub.sat.i64(i64 %506, i64 %507)
  store i64 %508, ptr %352, align 8, !noalias !15723
  %509 = load i64, ptr %353, align 8, !noalias !15723, !noundef !1740
  %510 = icmp slt i64 %508, %509
  br i1 %510, label %511, label %.preheader288

511:                                              ; preds = %504
  store i64 %508, ptr %353, align 8, !noalias !15723
  br label %.preheader288

.preheader288:                                    ; preds = %511, %504
  br label %512

512:                                              ; preds = %.preheader288, %515
  %513 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15723
  %514 = icmp slt i64 %513, 0
  br i1 %514, label %515, label %__rustc::__rust_dealloc (.exit121)

515:                                              ; preds = %512
  %516 = add nsw i64 %513, 1
  %517 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %513, i64 %516 acq_rel acquire, align 8, !noalias !15723
  %518 = extractvalue { i64, i1 } %517, 1
  br i1 %518, label %519, label %512

519:                                              ; preds = %515
  %520 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %507 monotonic, align 8, !noalias !15723
  %521 = call i64 @llvm.ssub.sat.i64(i64 %520, i64 %507)
  %522 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15723
  br label %523

523:                                              ; preds = %526, %519
  %524 = phi i64 [ %522, %519 ], [ %529, %526 ]
  %525 = icmp slt i64 %521, %524
  br i1 %525, label %526, label %530

526:                                              ; preds = %523
  %527 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %524, i64 %521 monotonic monotonic, align 8, !noalias !15723
  %528 = extractvalue { i64, i1 } %527, 1
  %529 = extractvalue { i64, i1 } %527, 0
  br i1 %528, label %530, label %523

530:                                              ; preds = %526, %523
  %531 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15723
  br label %__rustc::__rust_dealloc (.exit121)

__rustc::__rust_dealloc (.exit121): ; preds = %512, %530
  call void @free(ptr noundef nonnull %411) #92, !noalias !15723
  br label %533

532:                                              ; preds = %484, %481
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %35) #89
          to label %534 unwind label %273

533:                                              ; preds = %__rustc::__rust_dealloc (.exit121), %.loopexit164
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %35)
          to label %544 unwind label %540

534:                                              ; preds = %542, %540, %532
  %535 = phi i1 [ true, %532 ], [ true, %540 ], [ false, %542 ]
  %536 = phi { ptr, i32 } [ %482, %532 ], [ %541, %540 ], [ %543, %542 ]
  %537 = icmp eq i64 %404, 0
  br i1 %537, label %574, label %538

538:                                              ; preds = %534
  %539 = mul nuw i64 %404, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %405) ]
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %405, i64 noundef %539, i64 noundef range(i64 1, -9223372036854775807) 8) #92
  br label %574

540:                                              ; preds = %533
  %541 = landingpad { ptr, i32 }
          cleanup
  br label %534

542:                                              ; preds = %1155
  %543 = landingpad { ptr, i32 }
          cleanup
  br label %534

544:                                              ; preds = %533
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  %545 = icmp eq i64 %404, 0
  br i1 %545, label %581, label %546

546:                                              ; preds = %544
  %547 = mul nuw i64 %404, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %405) ]
  %548 = load i64, ptr %352, align 8, !noundef !1740
  %549 = call i64 @llvm.umin.i64(i64 %547, i64 9223372036854775807)
  %550 = call i64 @llvm.ssub.sat.i64(i64 %548, i64 %549)
  store i64 %550, ptr %352, align 8
  %551 = load i64, ptr %353, align 8, !noundef !1740
  %552 = icmp slt i64 %550, %551
  br i1 %552, label %553, label %.preheader287

553:                                              ; preds = %546
  store i64 %550, ptr %353, align 8
  br label %.preheader287

.preheader287:                                    ; preds = %553, %546
  br label %554

554:                                              ; preds = %.preheader287, %557
  %555 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %556 = icmp slt i64 %555, 0
  br i1 %556, label %557, label %__rustc::__rust_dealloc (.exit122)

557:                                              ; preds = %554
  %558 = add nsw i64 %555, 1
  %559 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %555, i64 %558 acq_rel acquire, align 8
  %560 = extractvalue { i64, i1 } %559, 1
  br i1 %560, label %561, label %554

561:                                              ; preds = %557
  %562 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %549 monotonic, align 8
  %563 = call i64 @llvm.ssub.sat.i64(i64 %562, i64 %549)
  %564 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %565

565:                                              ; preds = %568, %561
  %566 = phi i64 [ %564, %561 ], [ %571, %568 ]
  %567 = icmp slt i64 %563, %566
  br i1 %567, label %568, label %572

568:                                              ; preds = %565
  %569 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %566, i64 %563 monotonic monotonic, align 8
  %570 = extractvalue { i64, i1 } %569, 1
  %571 = extractvalue { i64, i1 } %569, 0
  br i1 %570, label %572, label %565

572:                                              ; preds = %568, %565
  %573 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit122)

__rustc::__rust_dealloc (.exit122): ; preds = %554, %572
  call void @free(ptr noundef nonnull %405) #92
  br label %581

574:                                              ; preds = %538, %534
  call void @llvm.experimental.noalias.scope.decl(metadata !15726)
  %575 = load ptr, ptr %344, align 8, !alias.scope !15726, !noundef !1740
  %576 = icmp eq ptr %575, null
  br i1 %576, label %1284, label %577

577:                                              ; preds = %574
  %578 = atomicrmw sub ptr %575, i64 1 release, align 8, !noalias !15729
  %579 = icmp eq i64 %578, 1
  br i1 %579, label %580, label %1284

580:                                              ; preds = %577
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %344) #91
  br label %1284

581:                                              ; preds = %__rustc::__rust_dealloc (.exit122), %544
  call void @llvm.experimental.noalias.scope.decl(metadata !15734)
  %582 = load ptr, ptr %344, align 8, !alias.scope !15734, !noundef !1740
  %583 = icmp eq ptr %582, null
  br i1 %583, label %588, label %584

584:                                              ; preds = %581
  %585 = atomicrmw sub ptr %582, i64 1 release, align 8, !noalias !15737
  %586 = icmp eq i64 %585, 1
  br i1 %586, label %587, label %588

587:                                              ; preds = %584
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %344) #91
  br label %588

588:                                              ; preds = %587, %584, %581
  call void @llvm.experimental.noalias.scope.decl(metadata !15742)
  %589 = load ptr, ptr %345, align 8, !alias.scope !15742, !noundef !1740
  %590 = icmp eq ptr %589, null
  br i1 %590, label %595, label %591

591:                                              ; preds = %588
  %592 = atomicrmw sub ptr %589, i64 1 release, align 8, !noalias !15745
  %593 = icmp eq i64 %592, 1
  br i1 %593, label %594, label %595

594:                                              ; preds = %591
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %345) #91
  br label %595

595:                                              ; preds = %594, %591, %588
  call void @llvm.lifetime.end.p0(ptr nonnull %36)
  %596 = icmp eq ptr %399, %293
  br i1 %596, label %.loopexit165, label %397

597:                                              ; preds = %630, %624, %620, %486
  call void @llvm.assume(i1 %420)
  %598 = icmp ugt i64 %489, %406
  br i1 %598, label %599, label %636, !prof !1742

599:                                              ; preds = %597
  call void @llvm.lifetime.start.p0(ptr nonnull %27)
  store i64 %489, ptr %27, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %26)
  store i64 %406, ptr %26, align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %25)
  store ptr %27, ptr %25, align 8
  %600 = getelementptr inbounds nuw i8, ptr %25, i64 8
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %600, align 8
  %601 = getelementptr inbounds nuw i8, ptr %25, i64 16
  store ptr %26, ptr %601, align 8
  %602 = getelementptr inbounds nuw i8, ptr %25, i64 24
  store ptr @<usize as core::fmt::Debug>::fmt, ptr %602, align 8
; invoke core::panicking::panic_fmt
  invoke void @core::panicking::panic_fmt(ptr noundef nonnull @anon.e5162873a9a3251d11c4df37a70e4654.2158, ptr noundef nonnull %25, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.289) #88
          to label %603 unwind label %479

603:                                              ; preds = %599
  unreachable

604:                                              ; preds = %486
  call void @llvm.lifetime.start.p0(ptr nonnull %24), !noalias !15750
  %605 = load atomic i64, ptr %311 monotonic, align 8, !noalias !15757
  br label %606

606:                                              ; preds = %606, %604
  %607 = phi i64 [ %605, %604 ], [ %611, %606 ]
  %608 = call i64 @llvm.uadd.sat.i64(i64 %607, i64 %494)
  %609 = cmpxchg weak ptr %311, i64 %607, i64 %608 monotonic monotonic, align 8, !noalias !15757
  %610 = extractvalue { i64, i1 } %609, 1
  %611 = extractvalue { i64, i1 } %609, 0
  br i1 %610, label %612, label %606

612:                                              ; preds = %606
  %613 = call i64 @llvm.uadd.sat.i64(i64 %611, i64 %494)
  %614 = load i64, ptr %277, align 8, !noalias !15757
  %615 = icmp ugt i64 %613, %614
  br i1 %615, label %616, label %620

616:                                              ; preds = %612
  call void @llvm.lifetime.start.p0(ptr nonnull %23), !noalias !15757
  store i8 0, ptr %312, align 1, !noalias !15757
  store i64 %614, ptr %313, align 8, !noalias !15757
  store i64 %613, ptr %314, align 8, !noalias !15757
  store i8 0, ptr %23, align 8, !noalias !15757
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %24, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %23)
          to label %617 unwind label %475

617:                                              ; preds = %616
  call void @llvm.lifetime.end.p0(ptr nonnull %23), !noalias !15757
  %618 = load i8, ptr %24, align 8, !noalias !15750
  %619 = icmp eq i8 %618, -1
  br i1 %619, label %620, label %623

620:                                              ; preds = %617, %612
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !15750
  %621 = load ptr, ptr %315, align 8, !noundef !1740
  %622 = icmp eq ptr %621, null
  br i1 %622, label %597, label %624

623:                                              ; preds = %617
  call void @llvm.lifetime.end.p0(ptr nonnull %24), !noalias !15750
  br label %.loopexit151

624:                                              ; preds = %620
  %625 = load i32, ptr %316, align 4, !noundef !1740
  %626 = getelementptr i8, ptr %621, i64 56
  %627 = load i64, ptr %626, align 8, !noundef !1740
  %628 = zext i32 %625 to i64
  %629 = icmp ugt i64 %627, %628
  br i1 %629, label %630, label %597

630:                                              ; preds = %624
  %631 = getelementptr i8, ptr %621, i64 48
  %632 = load ptr, ptr %631, align 8, !nonnull !1740, !noundef !1740
  %633 = getelementptr inbounds nuw [136 x i8], ptr %632, i64 %628
  %634 = getelementptr inbounds nuw [8 x i8], ptr %633, i64 %317
  %635 = atomicrmw add ptr %634, i64 %494 monotonic, align 8
  br label %597

636:                                              ; preds = %597
  %637 = icmp ult i64 %496, %489
  %638 = call i64 @llvm.umin.i64(i64 %496, i64 range(i64 0, 384307168202282326) %406)
  %639 = select i1 %637, i64 %489, i64 %638
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %405) ]
  %640 = icmp samesign ult i64 %639, %489
  br i1 %640, label %641, label %642, !prof !10952

641:                                              ; preds = %636
; invoke core::slice::index::slice_index_fail
  invoke void @core::slice::index::slice_index_fail(i64 noundef %489, i64 noundef %639, i64 noundef %406, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.e5162873a9a3251d11c4df37a70e4654.290) #93
          to label %1154 unwind label %479

642:                                              ; preds = %636
  %643 = mul nuw nsw i64 %489, 24
  %644 = getelementptr inbounds nuw i8, ptr %405, i64 %643
  %645 = mul nuw nsw i64 %639, 24
  %646 = getelementptr inbounds nuw i8, ptr %405, i64 %645
  %647 = icmp eq i64 %489, %639
  br i1 %647, label %.loopexit163, label %648

648:                                              ; preds = %642
  %649 = sub nuw nsw i64 %645, %643
  %650 = udiv exact i64 %649, 24
  br label %651

651:                                              ; preds = %666, %648
  %652 = phi i64 [ 0, %648 ], [ %667, %666 ]
  %653 = phi i64 [ 0, %648 ], [ %668, %666 ]
  %654 = phi i64 [ 0, %648 ], [ %669, %666 ]
  %655 = phi i64 [ 0, %648 ], [ %670, %666 ]
  %656 = getelementptr inbounds nuw [24 x i8], ptr %644, i64 %655
  %657 = load i8, ptr %656, align 8, !range !11184, !noalias !15760, !noundef !1740
  %658 = getelementptr i8, ptr %656, i64 8
  %659 = load i64, ptr %658, align 8, !noalias !15760
  switch i8 %657, label %.unreachabledefault [
    i8 0, label %660
    i8 1, label %662
    i8 2, label %666
    i8 3, label %664
  ]

.unreachabledefault:                              ; preds = %651
  unreachable

default.unreachable793:                           ; preds = %.preheader150
  unreachable

660:                                              ; preds = %651
  %661 = call i64 @llvm.uadd.sat.i64(i64 %654, i64 %659)
  br label %666

662:                                              ; preds = %651
  %663 = call i64 @llvm.uadd.sat.i64(i64 %653, i64 %659)
  br label %666

664:                                              ; preds = %651
  %665 = call i64 @llvm.umax.i64(i64 %652, i64 %659)
  br label %666

666:                                              ; preds = %664, %662, %660, %651
  %667 = phi i64 [ %652, %660 ], [ %652, %662 ], [ %665, %664 ], [ %652, %651 ]
  %668 = phi i64 [ %653, %660 ], [ %663, %662 ], [ %653, %664 ], [ %653, %651 ]
  %669 = phi i64 [ %661, %660 ], [ %654, %662 ], [ %654, %664 ], [ %654, %651 ]
  %670 = add nuw i64 %655, 1
  %671 = icmp eq i64 %670, %650
  br i1 %671, label %.loopexit163, label %651

.loopexit163:                                     ; preds = %666, %642
  %672 = phi i64 [ 0, %642 ], [ %669, %666 ]
  %673 = phi i64 [ 0, %642 ], [ %668, %666 ]
  %674 = phi i64 [ 0, %642 ], [ %667, %666 ]
  br i1 %282, label %678, label %.loopexit161

.loopexit161:                                     ; preds = %690, %678, %.loopexit163
  %675 = phi i64 [ 0, %.loopexit163 ], [ 0, %678 ], [ %692, %690 ]
  %676 = load atomic i32, ptr %318 acquire, align 8, !noalias !15764
  %677 = icmp eq i32 %676, 0
  br i1 %677, label %694, label %697

678:                                              ; preds = %.loopexit163
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %491) ]
  %679 = ptrtoint ptr %491 to i64
  %680 = call i64 @llvm.usub.sat.i64(i64 %498, i64 %492)
  %681 = sub nuw i64 %421, %679
  %682 = udiv exact i64 %681, 88
  %683 = call i64 @llvm.umin.i64(i64 %680, i64 %682)
  %684 = icmp eq i64 %683, 0
  br i1 %684, label %.loopexit161, label %.preheader160

.preheader160:                                    ; preds = %678, %690
  %685 = phi i64 [ %692, %690 ], [ 0, %678 ]
  %686 = phi i64 [ %691, %690 ], [ 0, %678 ]
  %687 = getelementptr inbounds nuw [88 x i8], ptr %491, i64 %686
  %688 = getelementptr inbounds nuw i8, ptr %687, i64 8
; invoke purrdf_sparql_eval::scratch::value_bytes
  %689 = invoke noundef i64 @purrdf_sparql_eval::scratch::value_bytes(ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(80) %688)
          to label %690 unwind label %473

690:                                              ; preds = %.preheader160
  %691 = add nuw nsw i64 %686, 1
  %692 = call noundef range(i64 32, 0) i64 @llvm.uadd.sat.i64(i64 %685, i64 %689)
  %693 = icmp eq i64 %691, %683
  br i1 %693, label %.loopexit161, label %.preheader160

694:                                              ; preds = %.loopexit161
  %695 = load i8, ptr %319, align 8
  %696 = icmp eq i8 %695, -1
  br i1 %696, label %697, label %704

697:                                              ; preds = %694, %.loopexit161
  br i1 %279, label %698, label %699

698:                                              ; preds = %699, %697
  br i1 %282, label %707, label %705

699:                                              ; preds = %697
  %700 = load atomic i64, ptr %311 monotonic, align 8, !noalias !15767
  %701 = load i64, ptr %277, align 8, !noalias !15767
  %702 = call i64 @llvm.uadd.sat.i64(i64 %700, i64 %672)
  %703 = icmp ugt i64 %702, %701
  br i1 %703, label %704, label %698

704:                                              ; preds = %707, %699, %694
  br i1 %647, label %.loopexit152, label %.preheader150

705:                                              ; preds = %707, %698
  %706 = or i1 %279, %647
  br i1 %706, label %.loopexit159, label %.preheader158

707:                                              ; preds = %698
  %708 = load i64, ptr %320, align 8, !noundef !1740
  %709 = load atomic i64, ptr %321 monotonic, align 16
  %710 = call noundef i64 @llvm.usub.sat.i64(i64 %708, i64 %709)
  %711 = call i64 @llvm.uadd.sat.i64(i64 %710, i64 %675)
  %712 = call i64 @llvm.uadd.sat.i64(i64 %711, i64 %673)
  %713 = call i64 @llvm.uadd.sat.i64(i64 %712, i64 %674)
  %714 = load atomic i64, ptr %322 monotonic, align 8, !noalias !15770
  %715 = load i64, ptr %280, align 8, !noalias !15770
  %716 = call i64 @llvm.uadd.sat.i64(i64 %714, i64 %713)
  %717 = icmp ugt i64 %716, %715
  br i1 %717, label %704, label %705

.preheader150:                                    ; preds = %704, %898
  %718 = phi ptr [ %899, %898 ], [ %488, %704 ]
  %719 = phi ptr [ %723, %898 ], [ %644, %704 ]
  %720 = phi ptr [ %902, %898 ], [ %491, %704 ]
  %721 = phi i64 [ %901, %898 ], [ %492, %704 ]
  %722 = phi ptr [ %900, %898 ], [ %488, %704 ]
  %723 = getelementptr inbounds nuw i8, ptr %719, i64 24
  %724 = load i8, ptr %719, align 8, !range !11184, !noundef !1740
  switch i8 %724, label %default.unreachable793 [
    i8 0, label %730
    i8 1, label %737
    i8 2, label %742
    i8 3, label %765
  ]

.loopexit152:                                     ; preds = %898, %704
  %725 = phi ptr [ %488, %704 ], [ %899, %898 ]
  %726 = phi i64 [ %492, %704 ], [ %901, %898 ]
  %727 = phi ptr [ %491, %704 ], [ %902, %898 ]
  %728 = icmp ult i64 %726, %498
  %729 = select i1 %282, i1 %728, i1 false
  br i1 %729, label %918, label %911

730:                                              ; preds = %.preheader150
  %731 = getelementptr inbounds nuw i8, ptr %719, i64 1
  %732 = load i8, ptr %731, align 1, !range !1741, !noundef !1740
  %733 = getelementptr inbounds nuw i8, ptr %719, i64 8
  %734 = load i64, ptr %733, align 8, !noundef !1740
  %735 = getelementptr inbounds nuw i8, ptr %719, i64 16
  %736 = load i64, ptr %735, align 8, !noundef !1740
  br i1 %279, label %898, label %766

737:                                              ; preds = %.preheader150
  %738 = getelementptr inbounds nuw i8, ptr %719, i64 8
  %739 = load i64, ptr %738, align 8, !noundef !1740
  %740 = getelementptr inbounds nuw i8, ptr %719, i64 16
  %741 = load i64, ptr %740, align 8, !noundef !1740
  br i1 %282, label %825, label %898

742:                                              ; preds = %.preheader150
  %743 = getelementptr inbounds nuw i8, ptr %719, i64 8
  %744 = load i64, ptr %743, align 8, !noundef !1740
  %745 = icmp ult i64 %721, %744
  br i1 %745, label %746, label %875

746:                                              ; preds = %742
  %747 = icmp eq ptr %720, %410
  br i1 %747, label %.loopexit144, label %748

748:                                              ; preds = %746
  %749 = add i64 %744, -1
  br label %753

750:                                              ; preds = %763
  %751 = add i64 %755, 1
  %752 = icmp eq ptr %756, %410
  br i1 %752, label %.loopexit144, label %753

753:                                              ; preds = %750, %748
  %754 = phi ptr [ %756, %750 ], [ %720, %748 ]
  %755 = phi i64 [ %751, %750 ], [ %721, %748 ]
  %756 = getelementptr inbounds nuw i8, ptr %754, i64 88
  %757 = getelementptr inbounds nuw i8, ptr %754, i64 8
  %758 = load i64, ptr %757, align 8, !noalias !15773
  %759 = icmp eq i64 %758, -1
  br i1 %759, label %.loopexit144, label %760

760:                                              ; preds = %753
  %761 = getelementptr inbounds nuw i8, ptr %754, i64 16
  %762 = load i64, ptr %754, align 8, !noalias !15773
  call void @llvm.lifetime.start.p0(ptr nonnull %22), !noalias !15776
  store i64 %758, ptr %22, align 8, !noalias !15776
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %326, ptr noundef nonnull align 8 dereferenceable(72) %761, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %327, i64 noundef %762, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %22)
          to label %763 unwind label %461

763:                                              ; preds = %760
  call void @llvm.lifetime.end.p0(ptr nonnull %22), !noalias !15776
  %764 = icmp eq i64 %755, %749
  br i1 %764, label %.loopexit144, label %750

765:                                              ; preds = %.preheader150
  br i1 %282, label %884, label %898

766:                                              ; preds = %730
  call void @llvm.lifetime.start.p0(ptr nonnull %21), !noalias !15779
  %767 = load atomic i64, ptr %311 monotonic, align 8, !noalias !15786
  br label %768

768:                                              ; preds = %768, %766
  %769 = phi i64 [ %767, %766 ], [ %773, %768 ]
  %770 = call i64 @llvm.uadd.sat.i64(i64 %769, i64 %734)
  %771 = cmpxchg weak ptr %311, i64 %769, i64 %770 monotonic monotonic, align 8, !noalias !15786
  %772 = extractvalue { i64, i1 } %771, 1
  %773 = extractvalue { i64, i1 } %771, 0
  br i1 %772, label %774, label %768

774:                                              ; preds = %768
  %775 = call i64 @llvm.uadd.sat.i64(i64 %773, i64 %734)
  %776 = load i64, ptr %277, align 8, !noalias !15786
  %777 = icmp ugt i64 %775, %776
  br i1 %777, label %778, label %787

778:                                              ; preds = %774
  call void @llvm.lifetime.start.p0(ptr nonnull %20), !noalias !15786
  store i8 0, ptr %332, align 1, !noalias !15786
  store i64 %776, ptr %333, align 8, !noalias !15786
  store i64 %775, ptr %334, align 8, !noalias !15786
  store i8 0, ptr %20, align 8, !noalias !15786
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %21, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %20)
          to label %779 unwind label %457

779:                                              ; preds = %778
  call void @llvm.lifetime.end.p0(ptr nonnull %20), !noalias !15786
  %780 = load i8, ptr %21, align 8, !noalias !15779
  %781 = icmp eq i8 %780, -1
  br i1 %781, label %787, label %782

782:                                              ; preds = %779
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !15779
  %783 = icmp ne i64 %736, 0
  %784 = add i64 %736, -1
  %785 = icmp ult i64 %721, %784
  %786 = select i1 %783, i1 %785, i1 false
  br i1 %786, label %789, label %.loopexit151

787:                                              ; preds = %779, %774
  call void @llvm.lifetime.end.p0(ptr nonnull %21), !noalias !15779
  %788 = icmp eq i8 %732, -1
  br i1 %788, label %898, label %808

789:                                              ; preds = %782
  %790 = icmp eq ptr %720, %410
  br i1 %790, label %.loopexit142, label %791

791:                                              ; preds = %789
  %792 = add i64 %736, -2
  br label %793

793:                                              ; preds = %803, %791
  %794 = phi ptr [ %796, %803 ], [ %720, %791 ]
  %795 = phi i64 [ %805, %803 ], [ %721, %791 ]
  %796 = getelementptr inbounds nuw i8, ptr %794, i64 88
  %797 = getelementptr inbounds nuw i8, ptr %794, i64 8
  %798 = load i64, ptr %797, align 8, !noalias !15789
  %799 = icmp eq i64 %798, -1
  br i1 %799, label %.loopexit142, label %800

800:                                              ; preds = %793
  %801 = getelementptr inbounds nuw i8, ptr %794, i64 16
  %802 = load i64, ptr %794, align 8, !noalias !15789
  call void @llvm.lifetime.start.p0(ptr nonnull %19), !noalias !15792
  store i64 %798, ptr %19, align 8, !noalias !15792
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %335, ptr noundef nonnull align 8 dereferenceable(72) %801, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %327, i64 noundef %802, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %19)
          to label %803 unwind label %455

803:                                              ; preds = %800
  call void @llvm.lifetime.end.p0(ptr nonnull %19), !noalias !15792
  %804 = icmp eq i64 %795, %792
  %805 = add nuw i64 %795, 1
  %806 = icmp eq ptr %796, %410
  %807 = select i1 %804, i1 true, i1 %806
  br i1 %807, label %.loopexit142, label %793

808:                                              ; preds = %787
  %809 = load ptr, ptr %315, align 8, !noundef !1740
  %810 = icmp eq ptr %809, null
  br i1 %810, label %898, label %811

811:                                              ; preds = %808
  %812 = load i32, ptr %316, align 4, !noundef !1740
  %813 = getelementptr i8, ptr %809, i64 56
  %814 = load i64, ptr %813, align 8, !noundef !1740
  %815 = zext i32 %812 to i64
  %816 = icmp ugt i64 %814, %815
  br i1 %816, label %817, label %898

817:                                              ; preds = %811
  %818 = getelementptr i8, ptr %809, i64 48
  %819 = load ptr, ptr %818, align 8, !nonnull !1740, !noundef !1740
  %820 = zext nneg i8 %732 to i64
  %821 = getelementptr inbounds nuw [136 x i8], ptr %819, i64 %815
  %822 = getelementptr inbounds nuw [8 x i8], ptr %821, i64 %820
  %823 = atomicrmw add ptr %822, i64 %734 monotonic, align 8
  br label %898

824:                                              ; preds = %828
  br i1 %829, label %.loopexit151, label %898

825:                                              ; preds = %737
  call void @llvm.lifetime.start.p0(ptr nonnull %31)
  %826 = load i64, ptr %280, align 8
  %827 = icmp eq i64 %826, -1
  br i1 %827, label %.thread136, label %832

.thread136:                                       ; preds = %843, %825
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  br label %898

828:                                              ; preds = %849, %847
  %.pr = load i8, ptr %31, align 8
  %829 = icmp ne i8 %.pr, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %31)
  %830 = icmp ne i64 %741, 0
  %831 = and i1 %830, %829
  br i1 %831, label %850, label %824

832:                                              ; preds = %825
  %833 = load atomic i32, ptr %318 acquire, align 8, !noalias !15795
  %834 = icmp eq i32 %833, 0
  br i1 %834, label %847, label %835

835:                                              ; preds = %832
  %836 = load atomic i64, ptr %322 monotonic, align 8, !noalias !15795
  br label %837

837:                                              ; preds = %837, %835
  %838 = phi i64 [ %836, %835 ], [ %842, %837 ]
  %839 = call i64 @llvm.uadd.sat.i64(i64 %838, i64 %739)
  %840 = cmpxchg weak ptr %322, i64 %838, i64 %839 monotonic monotonic, align 8, !noalias !15795
  %841 = extractvalue { i64, i1 } %840, 1
  %842 = extractvalue { i64, i1 } %840, 0
  br i1 %841, label %843, label %837

843:                                              ; preds = %837
  %844 = call i64 @llvm.uadd.sat.i64(i64 %842, i64 %739)
  %845 = load i64, ptr %280, align 8, !noalias !15795
  %846 = icmp ugt i64 %844, %845
  br i1 %846, label %848, label %.thread136

847:                                              ; preds = %832
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %31, ptr noundef nonnull align 8 dereferenceable(24) %319, i64 24, i1 false)
  br label %828

848:                                              ; preds = %843
  call void @llvm.lifetime.start.p0(ptr nonnull %11), !noalias !15795
  store i8 3, ptr %328, align 1, !noalias !15795
  store i64 %845, ptr %329, align 8, !noalias !15795
  store i64 %844, ptr %330, align 8, !noalias !15795
  store i8 0, ptr %11, align 8, !noalias !15795
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %31, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %11)
          to label %849 unwind label %471

849:                                              ; preds = %848
  call void @llvm.lifetime.end.p0(ptr nonnull %11), !noalias !15795
  br label %828

850:                                              ; preds = %828
  %851 = add i64 %741, -1
  %852 = icmp ult i64 %721, %851
  br i1 %852, label %853, label %.loopexit151

853:                                              ; preds = %850
  %854 = icmp eq ptr %720, %410
  br i1 %854, label %.loopexit142, label %855

855:                                              ; preds = %853
  %856 = add i64 %741, -2
  br label %857

857:                                              ; preds = %867, %855
  %858 = phi ptr [ %860, %867 ], [ %720, %855 ]
  %859 = phi i64 [ %869, %867 ], [ %721, %855 ]
  %860 = getelementptr inbounds nuw i8, ptr %858, i64 88
  %861 = getelementptr inbounds nuw i8, ptr %858, i64 8
  %862 = load i64, ptr %861, align 8, !noalias !15798
  %863 = icmp eq i64 %862, -1
  br i1 %863, label %.loopexit142, label %864

864:                                              ; preds = %857
  %865 = getelementptr inbounds nuw i8, ptr %858, i64 16
  %866 = load i64, ptr %858, align 8, !noalias !15798
  call void @llvm.lifetime.start.p0(ptr nonnull %18), !noalias !15801
  store i64 %862, ptr %18, align 8, !noalias !15801
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %331, ptr noundef nonnull align 8 dereferenceable(72) %865, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %327, i64 noundef %866, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %18)
          to label %867 unwind label %459

867:                                              ; preds = %864
  call void @llvm.lifetime.end.p0(ptr nonnull %18), !noalias !15801
  %868 = icmp eq i64 %859, %856
  %869 = add nuw i64 %859, 1
  %870 = icmp eq ptr %860, %410
  %871 = select i1 %868, i1 true, i1 %870
  br i1 %871, label %.loopexit142, label %857

.loopexit144:                                     ; preds = %763, %753, %750, %746
  %872 = phi ptr [ %722, %746 ], [ %756, %750 ], [ %756, %753 ], [ %756, %763 ]
  %873 = phi i64 [ %721, %746 ], [ %744, %763 ], [ %755, %753 ], [ %751, %750 ]
  %874 = phi ptr [ %720, %746 ], [ %756, %750 ], [ %756, %753 ], [ %756, %763 ]
  store ptr %872, ptr %305, align 8
  br label %875

875:                                              ; preds = %.loopexit144, %742
  %876 = phi ptr [ %718, %742 ], [ %872, %.loopexit144 ]
  %877 = phi ptr [ %722, %742 ], [ %872, %.loopexit144 ]
  %878 = phi i64 [ %721, %742 ], [ %873, %.loopexit144 ]
  %879 = phi ptr [ %720, %742 ], [ %874, %.loopexit144 ]
  br i1 %282, label %880, label %898

880:                                              ; preds = %875
  call void @llvm.lifetime.start.p0(ptr nonnull %30)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %30, ptr noundef nonnull align 16 %1)
          to label %881 unwind label %471

881:                                              ; preds = %880
  %882 = load i8, ptr %30, align 8, !range !1743, !noundef !1740
  %883 = icmp eq i8 %882, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %30)
  br i1 %883, label %898, label %.loopexit151

884:                                              ; preds = %765
  %885 = getelementptr inbounds nuw i8, ptr %719, i64 8
  %886 = load i64, ptr %885, align 8, !noundef !1740
  call void @llvm.lifetime.start.p0(ptr nonnull %29)
  %887 = load atomic i32, ptr %318 acquire, align 8, !noalias !15804
  %888 = icmp eq i32 %887, 0
  br i1 %888, label %889, label %890

889:                                              ; preds = %884
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %29, ptr noundef nonnull align 8 dereferenceable(24) %319, i64 24, i1 false)
  br label %904

890:                                              ; preds = %884
  %891 = load atomic i64, ptr %322 monotonic, align 8, !noalias !15804
  %892 = call i64 @llvm.uadd.sat.i64(i64 %891, i64 %886)
  %893 = load i64, ptr %280, align 8, !noalias !15804
  %894 = icmp ugt i64 %892, %893
  br i1 %894, label %896, label %895

895:                                              ; preds = %890
  call void @llvm.lifetime.end.p0(ptr nonnull %29)
  br label %898

896:                                              ; preds = %890
  call void @llvm.lifetime.start.p0(ptr nonnull %17), !noalias !15804
  store i8 3, ptr %323, align 1, !noalias !15804
  store i64 %893, ptr %324, align 8, !noalias !15804
  store i64 %892, ptr %325, align 8, !noalias !15804
  store i8 0, ptr %17, align 8, !noalias !15804
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef align 8 captures(none) dereferenceable(24) %29, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %17)
          to label %897 unwind label %471

897:                                              ; preds = %896
  call void @llvm.lifetime.end.p0(ptr nonnull %17), !noalias !15804
  br label %904

898:                                              ; preds = %.thread136, %904, %895, %881, %875, %824, %817, %811, %808, %787, %765, %737, %730
  %899 = phi ptr [ %718, %824 ], [ %876, %875 ], [ %718, %765 ], [ %718, %730 ], [ %718, %737 ], [ %718, %895 ], [ %876, %881 ], [ %718, %904 ], [ %718, %787 ], [ %718, %808 ], [ %718, %817 ], [ %718, %811 ], [ %718, %.thread136 ]
  %900 = phi ptr [ %722, %824 ], [ %877, %875 ], [ %722, %765 ], [ %722, %730 ], [ %722, %737 ], [ %722, %895 ], [ %877, %881 ], [ %722, %904 ], [ %722, %787 ], [ %722, %808 ], [ %722, %817 ], [ %722, %811 ], [ %722, %.thread136 ]
  %901 = phi i64 [ %721, %824 ], [ %878, %875 ], [ %721, %765 ], [ %721, %730 ], [ %721, %737 ], [ %721, %895 ], [ %878, %881 ], [ %721, %904 ], [ %721, %787 ], [ %721, %808 ], [ %721, %817 ], [ %721, %811 ], [ %721, %.thread136 ]
  %902 = phi ptr [ %720, %824 ], [ %879, %875 ], [ %720, %765 ], [ %720, %730 ], [ %720, %737 ], [ %720, %895 ], [ %879, %881 ], [ %720, %904 ], [ %720, %787 ], [ %720, %808 ], [ %720, %817 ], [ %720, %811 ], [ %720, %.thread136 ]
  %903 = icmp eq ptr %723, %646
  br i1 %903, label %.loopexit152, label %.preheader150

904:                                              ; preds = %897, %889
  %905 = load i8, ptr %29, align 8
  %906 = icmp eq i8 %905, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %29)
  br i1 %906, label %898, label %.loopexit151

.loopexit142:                                     ; preds = %867, %857, %803, %793, %853, %789
  %907 = phi ptr [ %722, %853 ], [ %722, %789 ], [ %796, %803 ], [ %796, %793 ], [ %860, %857 ], [ %860, %867 ]
  store ptr %907, ptr %305, align 8
  br label %.loopexit151

.loopexit148:                                     ; preds = %935, %925, %922, %918
  %908 = phi ptr [ %725, %918 ], [ %928, %922 ], [ %928, %925 ], [ %928, %935 ]
  %909 = phi i64 [ %726, %918 ], [ %498, %935 ], [ %927, %925 ], [ %923, %922 ]
  %910 = phi ptr [ %727, %918 ], [ %928, %922 ], [ %928, %925 ], [ %928, %935 ]
  store ptr %908, ptr %305, align 8
  br label %911

911:                                              ; preds = %1029, %1004, %.loopexit148, %.loopexit152
  %912 = phi ptr [ %999, %1004 ], [ %1030, %1029 ], [ %725, %.loopexit152 ], [ %908, %.loopexit148 ]
  %913 = phi i64 [ %1000, %1004 ], [ %1031, %1029 ], [ %726, %.loopexit152 ], [ %909, %.loopexit148 ]
  %914 = phi ptr [ %1001, %1004 ], [ %1032, %1029 ], [ %727, %.loopexit152 ], [ %910, %.loopexit148 ]
  %915 = icmp eq i64 %500, 0
  br i1 %915, label %.loopexit146, label %916

916:                                              ; preds = %911
  %917 = load ptr, ptr %143, align 8, !alias.scope !15807, !noalias !15810, !nonnull !1740, !noundef !1740
  br label %1091

918:                                              ; preds = %.loopexit152
  %919 = icmp eq ptr %727, %410
  br i1 %919, label %.loopexit148, label %920

920:                                              ; preds = %918
  %921 = add i64 %498, -1
  br label %925

922:                                              ; preds = %935
  %923 = add i64 %927, 1
  %924 = icmp eq ptr %928, %410
  br i1 %924, label %.loopexit148, label %925

925:                                              ; preds = %922, %920
  %926 = phi ptr [ %928, %922 ], [ %727, %920 ]
  %927 = phi i64 [ %923, %922 ], [ %726, %920 ]
  %928 = getelementptr inbounds nuw i8, ptr %926, i64 88
  %929 = getelementptr inbounds nuw i8, ptr %926, i64 8
  %930 = load i64, ptr %929, align 8, !noalias !15812
  %931 = icmp eq i64 %930, -1
  br i1 %931, label %.loopexit148, label %932

932:                                              ; preds = %925
  %933 = getelementptr inbounds nuw i8, ptr %926, i64 16
  %934 = load i64, ptr %926, align 8, !noalias !15812
  call void @llvm.lifetime.start.p0(ptr nonnull %16), !noalias !15815
  store i64 %930, ptr %16, align 8, !noalias !15815
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %336, ptr noundef nonnull align 8 dereferenceable(72) %933, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %327, i64 noundef %934, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %16)
          to label %935 unwind label %469

935:                                              ; preds = %932
  call void @llvm.lifetime.end.p0(ptr nonnull %16), !noalias !15815
  %936 = icmp eq i64 %927, %921
  br i1 %936, label %.loopexit148, label %922

.loopexit159:                                     ; preds = %952, %705
  %937 = icmp eq i64 %489, %639
  br i1 %937, label %.loopexit157, label %.lr.ph

938:                                              ; preds = %.lr.ph
  %939 = icmp eq ptr %644, %941
  br i1 %939, label %.loopexit157, label %.lr.ph

.lr.ph:                                           ; preds = %.loopexit159, %938
  %940 = phi ptr [ %941, %938 ], [ %646, %.loopexit159 ]
  %941 = getelementptr inbounds i8, ptr %940, i64 -24
  %942 = load i8, ptr %941, align 8, !range !11184, !noalias !15818, !noundef !1740
  %943 = icmp eq i8 %942, 2
  br i1 %943, label %972, label %938

.preheader158:                                    ; preds = %705, %952
  %944 = phi ptr [ %945, %952 ], [ %644, %705 ]
  %945 = getelementptr inbounds nuw i8, ptr %944, i64 24
  %946 = load i8, ptr %944, align 8, !range !11184, !noundef !1740
  %947 = icmp eq i8 %946, 0
  br i1 %947, label %948, label %952

948:                                              ; preds = %.preheader158
  %949 = getelementptr inbounds nuw i8, ptr %944, i64 1
  %950 = load i8, ptr %949, align 1, !range !1741, !noundef !1740
  %951 = icmp eq i8 %950, -1
  br i1 %951, label %952, label %954

952:                                              ; preds = %963, %957, %954, %948, %.preheader158
  %953 = icmp eq ptr %945, %646
  br i1 %953, label %.loopexit159, label %.preheader158

954:                                              ; preds = %948
  %955 = load ptr, ptr %315, align 8, !noundef !1740
  %956 = icmp eq ptr %955, null
  br i1 %956, label %952, label %957

957:                                              ; preds = %954
  %958 = load i32, ptr %316, align 4, !noundef !1740
  %959 = getelementptr i8, ptr %955, i64 56
  %960 = load i64, ptr %959, align 8, !noundef !1740
  %961 = zext i32 %958 to i64
  %962 = icmp ugt i64 %960, %961
  br i1 %962, label %963, label %952

963:                                              ; preds = %957
  %964 = getelementptr i8, ptr %955, i64 48
  %965 = load ptr, ptr %964, align 8, !nonnull !1740, !noundef !1740
  %966 = getelementptr inbounds nuw i8, ptr %944, i64 8
  %967 = load i64, ptr %966, align 8, !noundef !1740
  %968 = zext nneg i8 %950 to i64
  %969 = getelementptr inbounds nuw [136 x i8], ptr %965, i64 %961
  %970 = getelementptr inbounds nuw [8 x i8], ptr %969, i64 %968
  %971 = atomicrmw add ptr %970, i64 %967 monotonic, align 8
  br label %952

972:                                              ; preds = %.lr.ph
  %973 = getelementptr i8, ptr %940, i64 -16
  %974 = load i64, ptr %973, align 8, !noalias !15818
  %975 = icmp ult i64 %492, %974
  br i1 %975, label %976, label %.loopexit157

976:                                              ; preds = %972
  %977 = icmp eq ptr %491, %410
  br i1 %977, label %.loopexit155, label %978

978:                                              ; preds = %976
  %979 = add i64 %974, -1
  br label %983

980:                                              ; preds = %993
  %981 = add i64 %985, 1
  %982 = icmp eq ptr %986, %410
  br i1 %982, label %.loopexit155, label %983

983:                                              ; preds = %980, %978
  %984 = phi ptr [ %986, %980 ], [ %491, %978 ]
  %985 = phi i64 [ %981, %980 ], [ %492, %978 ]
  %986 = getelementptr inbounds nuw i8, ptr %984, i64 88
  %987 = getelementptr inbounds nuw i8, ptr %984, i64 8
  %988 = load i64, ptr %987, align 8, !noalias !15821
  %989 = icmp eq i64 %988, -1
  br i1 %989, label %.loopexit155, label %990

990:                                              ; preds = %983
  %991 = getelementptr inbounds nuw i8, ptr %984, i64 16
  %992 = load i64, ptr %984, align 8, !noalias !15821
  call void @llvm.lifetime.start.p0(ptr nonnull %15), !noalias !15824
  store i64 %988, ptr %15, align 8, !noalias !15824
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %337, ptr noundef nonnull align 8 dereferenceable(72) %991, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %327, i64 noundef %992, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %15)
          to label %993 unwind label %467

993:                                              ; preds = %990
  call void @llvm.lifetime.end.p0(ptr nonnull %15), !noalias !15824
  %994 = icmp eq i64 %985, %979
  br i1 %994, label %.loopexit155, label %980

.loopexit155:                                     ; preds = %993, %983, %980, %976
  %995 = phi ptr [ %488, %976 ], [ %986, %980 ], [ %986, %983 ], [ %986, %993 ]
  %996 = phi i64 [ %492, %976 ], [ %974, %993 ], [ %985, %983 ], [ %981, %980 ]
  %997 = phi ptr [ %491, %976 ], [ %986, %980 ], [ %986, %983 ], [ %986, %993 ]
  store ptr %995, ptr %305, align 8
  br label %.loopexit157

.loopexit157:                                     ; preds = %938, %.loopexit159, %.loopexit155, %972
  %998 = phi i1 [ false, %.loopexit155 ], [ false, %972 ], [ true, %.loopexit159 ], [ true, %938 ]
  %999 = phi ptr [ %995, %.loopexit155 ], [ %488, %972 ], [ %488, %.loopexit159 ], [ %488, %938 ]
  %1000 = phi i64 [ %996, %.loopexit155 ], [ %492, %972 ], [ %492, %.loopexit159 ], [ %492, %938 ]
  %1001 = phi ptr [ %997, %.loopexit155 ], [ %491, %972 ], [ %491, %.loopexit159 ], [ %491, %938 ]
  %1002 = icmp eq i64 %672, 0
  %1003 = or i1 %279, %1002
  br i1 %1003, label %1004, label %1005

1004:                                             ; preds = %1021, %.loopexit157
  br i1 %282, label %1023, label %911

1005:                                             ; preds = %.loopexit157
  call void @llvm.lifetime.start.p0(ptr nonnull %14), !noalias !15827
  %1006 = load atomic i64, ptr %311 monotonic, align 8, !noalias !15834
  br label %1007

1007:                                             ; preds = %1007, %1005
  %1008 = phi i64 [ %1006, %1005 ], [ %1012, %1007 ]
  %1009 = call i64 @llvm.uadd.sat.i64(i64 %1008, i64 %672)
  %1010 = cmpxchg weak ptr %311, i64 %1008, i64 %1009 monotonic monotonic, align 8, !noalias !15834
  %1011 = extractvalue { i64, i1 } %1010, 1
  %1012 = extractvalue { i64, i1 } %1010, 0
  br i1 %1011, label %1013, label %1007

1013:                                             ; preds = %1007
  %1014 = call i64 @llvm.uadd.sat.i64(i64 %1012, i64 %672)
  %1015 = load i64, ptr %277, align 8, !noalias !15834
  %1016 = icmp ugt i64 %1014, %1015
  br i1 %1016, label %1017, label %1021

1017:                                             ; preds = %1013
  call void @llvm.lifetime.start.p0(ptr nonnull %13), !noalias !15834
  store i8 0, ptr %338, align 1, !noalias !15834
  store i64 %1015, ptr %339, align 8, !noalias !15834
  store i64 %1014, ptr %340, align 8, !noalias !15834
  store i8 0, ptr %13, align 8, !noalias !15834
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %14, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %13)
          to label %1018 unwind label %465

1018:                                             ; preds = %1017
  call void @llvm.lifetime.end.p0(ptr nonnull %13), !noalias !15834
  %1019 = load i8, ptr %14, align 8, !noalias !15827
  %1020 = icmp eq i8 %1019, -1
  br i1 %1020, label %1021, label %1022

1021:                                             ; preds = %1018, %1013
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !15827
  br label %1004

1022:                                             ; preds = %1018
  call void @llvm.lifetime.end.p0(ptr nonnull %14), !noalias !15827
  br i1 %282, label %1069, label %.loopexit151

1023:                                             ; preds = %1004
  call void @llvm.lifetime.start.p0(ptr nonnull %34)
  %1024 = load i64, ptr %280, align 8
  %1025 = icmp eq i64 %1024, -1
  br i1 %1025, label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread), label %1033

.loopexit153:                                     ; preds = %1089, %1079, %1076, %1072
  %1026 = phi ptr [ %999, %1072 ], [ %1082, %1076 ], [ %1082, %1079 ], [ %1082, %1089 ]
  %1027 = phi i64 [ %1000, %1072 ], [ %498, %1089 ], [ %1081, %1079 ], [ %1077, %1076 ]
  %1028 = phi ptr [ %1001, %1072 ], [ %1082, %1076 ], [ %1082, %1079 ], [ %1082, %1089 ]
  store ptr %1026, ptr %305, align 8
  br label %1029

1029:                                             ; preds = %1069, %.loopexit153
  %1030 = phi ptr [ %999, %1069 ], [ %1026, %.loopexit153 ]
  %1031 = phi i64 [ %1000, %1069 ], [ %1027, %.loopexit153 ]
  %1032 = phi ptr [ %1001, %1069 ], [ %1028, %.loopexit153 ]
  br i1 %1070, label %.loopexit151, label %911

1033:                                             ; preds = %1023
  %1034 = load atomic i32, ptr %318 acquire, align 8, !noalias !15837
  %1035 = icmp eq i32 %1034, 0
  br i1 %1035, label %1047, label %1036

1036:                                             ; preds = %1033
  %1037 = load atomic i64, ptr %322 monotonic, align 8, !noalias !15837
  br label %1038

1038:                                             ; preds = %1038, %1036
  %1039 = phi i64 [ %1037, %1036 ], [ %1043, %1038 ]
  %1040 = call i64 @llvm.uadd.sat.i64(i64 %1039, i64 %673)
  %1041 = cmpxchg weak ptr %322, i64 %1039, i64 %1040 monotonic monotonic, align 8, !noalias !15837
  %1042 = extractvalue { i64, i1 } %1041, 1
  %1043 = extractvalue { i64, i1 } %1041, 0
  br i1 %1042, label %1044, label %1038

1044:                                             ; preds = %1038
  %1045 = call i64 @llvm.uadd.sat.i64(i64 %1043, i64 %673)
  %.sroa.3130.0.copyload = load i64, ptr %280, align 8, !noalias !15837
  %1046 = icmp ugt i64 %1045, %.sroa.3130.0.copyload
  br i1 %1046, label %1048, label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread)

1047:                                             ; preds = %1033
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %34, ptr noundef nonnull align 8 dereferenceable(24) %319, i64 24, i1 false)
  br label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit)

1048:                                             ; preds = %1044
  call void @llvm.lifetime.start.p0(ptr nonnull %8), !noalias !15837
  store i8 3, ptr %346, align 1, !noalias !15837
  store i64 %.sroa.3130.0.copyload, ptr %347, align 8, !noalias !15837
  store i64 %1045, ptr %348, align 8, !noalias !15837
  store i8 0, ptr %8, align 8, !noalias !15837
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %34, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %8)
          to label %.noexc123 unwind label %477

.noexc123:                                        ; preds = %1048
  call void @llvm.lifetime.end.p0(ptr nonnull %8), !noalias !15837
  br label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit)

<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread): ; preds = %1044, %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit), %1023
  call void @llvm.lifetime.end.p0(ptr nonnull %34)
  br i1 %998, label %1051, label %1053

<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit): ; preds = %.noexc123, %1047
  %.pr135 = load i8, ptr %34, align 8
  %1049 = icmp eq i8 %.pr135, -1
  br i1 %1049, label %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread), label %1050

1050:                                             ; preds = %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit)
  call void @llvm.lifetime.end.p0(ptr nonnull %34)
  br label %1069

1051:                                             ; preds = %1054, %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread)
  %1052 = icmp eq i64 %674, 0
  br i1 %1052, label %1069, label %1057

1053:                                             ; preds = %<purrdf_sparql_eval::governor::GovernorState>::charge_work (.llvm.12908414067662811932.exit.thread)
  call void @llvm.lifetime.start.p0(ptr nonnull %33)
; invoke <purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth
  invoke fastcc void @<purrdf_sparql_eval::eval::EvalCtx>::charge_scratch_growth(ptr noalias nofree noundef align 8 captures(address) dereferenceable(24) %33, ptr noundef nonnull align 16 %1)
          to label %1054 unwind label %477

1054:                                             ; preds = %1053
  %1055 = load i8, ptr %33, align 8, !range !1743, !noundef !1740
  %1056 = icmp eq i8 %1055, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %33)
  br i1 %1056, label %1051, label %1069

1057:                                             ; preds = %1051
  call void @llvm.lifetime.start.p0(ptr nonnull %32)
  call void @llvm.experimental.noalias.scope.decl(metadata !15840)
  %1058 = load atomic i32, ptr %318 acquire, align 8, !noalias !15840
  %1059 = icmp eq i32 %1058, 0
  br i1 %1059, label %1060, label %1061

1060:                                             ; preds = %1057
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %32, ptr noundef nonnull align 8 dereferenceable(24) %319, i64 24, i1 false)
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

1061:                                             ; preds = %1057
  %1062 = load atomic i64, ptr %322 monotonic, align 8, !noalias !15840
  %1063 = call i64 @llvm.uadd.sat.i64(i64 %1062, i64 %674)
  %.sroa.3133.0.copyload = load i64, ptr %280, align 8, !noalias !15840
  %1064 = icmp ugt i64 %1063, %.sroa.3133.0.copyload
  br i1 %1064, label %1066, label %1065

1065:                                             ; preds = %1061
  store i8 -1, ptr %32, align 8, !alias.scope !15840
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

1066:                                             ; preds = %1061
  call void @llvm.lifetime.start.p0(ptr nonnull %7), !noalias !15840
  store i8 3, ptr %349, align 1, !noalias !15840
  store i64 %.sroa.3133.0.copyload, ptr %350, align 8, !noalias !15840
  store i64 %1063, ptr %351, align 8, !noalias !15840
  store i8 0, ptr %7, align 8, !noalias !15840
; invoke <purrdf_sparql_eval::governor::GovernorState>::trip
  invoke fastcc void @<purrdf_sparql_eval::governor::GovernorState>::trip (.llvm.12908414067662811932)(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(24) %32, ptr noundef nonnull align 8 %277, ptr noalias nofree noundef readonly align 8 captures(none) dereferenceable(24) %7)
          to label %.noexc124 unwind label %477

.noexc124:                                        ; preds = %1066
  call void @llvm.lifetime.end.p0(ptr nonnull %7), !noalias !15840
  br label %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit)

<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit): ; preds = %.noexc124, %1065, %1060
  %1067 = load i8, ptr %32, align 8, !range !1743, !noundef !1740
  %1068 = icmp ne i8 %1067, -1
  call void @llvm.lifetime.end.p0(ptr nonnull %32)
  br label %1069

1069:                                             ; preds = %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit), %1054, %1051, %1050, %1022
  %1070 = phi i1 [ false, %1051 ], [ %1068, %<purrdf_sparql_eval::governor::GovernorState>::admit_transient (.exit) ], [ true, %1022 ], [ true, %1050 ], [ true, %1054 ]
  %1071 = icmp ult i64 %1000, %498
  br i1 %1071, label %1072, label %1029

1072:                                             ; preds = %1069
  %1073 = icmp eq ptr %1001, %410
  br i1 %1073, label %.loopexit153, label %1074

1074:                                             ; preds = %1072
  %1075 = add i64 %498, -1
  br label %1079

1076:                                             ; preds = %1089
  %1077 = add i64 %1081, 1
  %1078 = icmp eq ptr %1082, %410
  br i1 %1078, label %.loopexit153, label %1079

1079:                                             ; preds = %1076, %1074
  %1080 = phi ptr [ %1082, %1076 ], [ %1001, %1074 ]
  %1081 = phi i64 [ %1077, %1076 ], [ %1000, %1074 ]
  %1082 = getelementptr inbounds nuw i8, ptr %1080, i64 88
  %1083 = getelementptr inbounds nuw i8, ptr %1080, i64 8
  %1084 = load i64, ptr %1083, align 8, !noalias !15843
  %1085 = icmp eq i64 %1084, -1
  br i1 %1085, label %.loopexit153, label %1086

1086:                                             ; preds = %1079
  %1087 = getelementptr inbounds nuw i8, ptr %1080, i64 16
  %1088 = load i64, ptr %1080, align 8, !noalias !15843
  call void @llvm.lifetime.start.p0(ptr nonnull %12), !noalias !15846
  store i64 %1084, ptr %12, align 8, !noalias !15846
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(72) %341, ptr noundef nonnull align 8 dereferenceable(72) %1087, i64 72, i1 false)
; invoke <purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint
  invoke void @<purrdf_sparql_eval::scratch::ScratchInterner>::count_worker_mint(ptr noalias nofree noundef nonnull align 8 dereferenceable(184) %327, i64 noundef %1088, ptr noalias nofree noundef nonnull align 8 captures(address) dereferenceable(80) %12)
          to label %1089 unwind label %463

1089:                                             ; preds = %1086
  call void @llvm.lifetime.end.p0(ptr nonnull %12), !noalias !15846
  %1090 = icmp eq i64 %1081, %1075
  br i1 %1090, label %.loopexit153, label %1076

1091:                                             ; preds = %1116, %916
  %1092 = phi i64 [ %500, %916 ], [ %1094, %1116 ]
  %1093 = phi ptr [ %487, %916 ], [ %1099, %1116 ]
  %1094 = add i64 %1092, -1
  call void @llvm.experimental.noalias.scope.decl(metadata !15807)
  %1095 = icmp eq ptr %1093, %917
  br i1 %1095, label %.loopexit146, label %1098

.loopexit146:                                     ; preds = %1116, %1098, %1091, %911
  %1096 = phi ptr [ %487, %911 ], [ %1099, %1098 ], [ %1099, %1116 ], [ %1093, %1091 ]
  store ptr %1096, ptr %142, align 8
  %1097 = icmp eq ptr %493, %416
  br i1 %1097, label %.loopexit164, label %486

1098:                                             ; preds = %1091
  %1099 = getelementptr inbounds nuw i8, ptr %1093, i64 40
  %1100 = load i64, ptr %1093, align 8, !noalias !15807
  %1101 = getelementptr inbounds nuw i8, ptr %1093, i64 8
  %1102 = load ptr, ptr %1101, align 8, !noalias !15807
  %1103 = icmp eq i64 %1100, 0
  br i1 %1103, label %.loopexit146, label %1104

1104:                                             ; preds = %1098
  %1105 = getelementptr inbounds nuw i8, ptr %1093, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %28, ptr noundef nonnull align 8 dereferenceable(24) %1105, i64 24, i1 false)
  call void @llvm.experimental.noalias.scope.decl(metadata !15849)
  %1106 = load i64, ptr %342, align 8, !alias.scope !15849, !noalias !15852, !noundef !1740
  %1107 = load i64, ptr %45, align 8, !range !1835, !alias.scope !15849, !noalias !15852, !noundef !1740
  %1108 = icmp eq i64 %1106, %1107
  br i1 %1108, label %1109, label %1116

1109:                                             ; preds = %1104
; invoke <alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one
  invoke void @<alloc::raw_vec::RawVec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>::grow_one(ptr noalias nofree noundef nonnull align 8 dereferenceable(24) %45)
          to label %1116 unwind label %1110, !noalias !15852

1110:                                             ; preds = %1109
  %1111 = landingpad { ptr, i32 }
          cleanup
  store ptr %1099, ptr %142, align 8
  %1112 = icmp ugt i64 %1100, 5
  br i1 %1112, label %1113, label %481

1113:                                             ; preds = %1110
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %1102) ]
  %1114 = shl i64 %1100, 3
  %1115 = add i64 %1114, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %1102, i64 noundef %1115, i64 noundef range(i64 1, -9223372036854775807) 4) #92, !noalias !15854
  br label %481

1116:                                             ; preds = %1109, %1104
  %1117 = load ptr, ptr %343, align 8, !alias.scope !15849, !noalias !15852, !nonnull !1740, !noundef !1740
  %1118 = getelementptr inbounds nuw [40 x i8], ptr %1117, i64 %1106
  store i64 %1100, ptr %1118, align 8, !noalias !15849
  %1119 = getelementptr inbounds nuw i8, ptr %1118, i64 8
  store ptr %1102, ptr %1119, align 8, !noalias !15849
  %1120 = getelementptr inbounds nuw i8, ptr %1118, i64 16
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1120, ptr noundef nonnull align 8 dereferenceable(24) %28, i64 24, i1 false), !noalias !15849
  %1121 = add i64 %1106, 1
  store i64 %1121, ptr %342, align 8, !alias.scope !15849, !noalias !15852
  %1122 = icmp eq i64 %1094, 0
  br i1 %1122, label %.loopexit146, label %1091

.loopexit151:                                     ; preds = %1022, %1029, %824, %881, %904, %782, %850, %.loopexit142, %623
  %.sink = phi i8 [ 1, %782 ], [ 0, %623 ], [ 1, %850 ], [ 1, %.loopexit142 ], [ 1, %824 ], [ 1, %904 ], [ 1, %881 ], [ 1, %1029 ], [ 1, %1022 ]
  %1123 = getelementptr inbounds nuw i8, ptr %0, i64 8
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(24) %1123, ptr noundef nonnull align 8 dereferenceable(24) %45, i64 24, i1 false)
  %1124 = getelementptr inbounds nuw i8, ptr %0, i64 32
  store i8 %.sink, ptr %1124, align 16
  store i64 -1, ptr %0, align 16
  %1125 = icmp eq i64 %412, 0
  br i1 %1125, label %1155, label %1126

1126:                                             ; preds = %.loopexit151
  %1127 = shl nuw i64 %412, 5
  %1128 = load i64, ptr %352, align 8, !noalias !15857, !noundef !1740
  %1129 = call i64 @llvm.umin.i64(i64 %1127, i64 9223372036854775807)
  %1130 = call i64 @llvm.ssub.sat.i64(i64 %1128, i64 %1129)
  store i64 %1130, ptr %352, align 8, !noalias !15857
  %1131 = load i64, ptr %353, align 8, !noalias !15857, !noundef !1740
  %1132 = icmp slt i64 %1130, %1131
  br i1 %1132, label %1133, label %.preheader276

1133:                                             ; preds = %1126
  store i64 %1130, ptr %353, align 8, !noalias !15857
  br label %.preheader276

.preheader276:                                    ; preds = %1133, %1126
  br label %1134

1134:                                             ; preds = %.preheader276, %1137
  %1135 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15857
  %1136 = icmp slt i64 %1135, 0
  br i1 %1136, label %1137, label %__rustc::__rust_dealloc (.exit125)

1137:                                             ; preds = %1134
  %1138 = add nsw i64 %1135, 1
  %1139 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1135, i64 %1138 acq_rel acquire, align 8, !noalias !15857
  %1140 = extractvalue { i64, i1 } %1139, 1
  br i1 %1140, label %1141, label %1134

1141:                                             ; preds = %1137
  %1142 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1129 monotonic, align 8, !noalias !15857
  %1143 = call i64 @llvm.ssub.sat.i64(i64 %1142, i64 %1129)
  %1144 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15857
  br label %1145

1145:                                             ; preds = %1148, %1141
  %1146 = phi i64 [ %1144, %1141 ], [ %1151, %1148 ]
  %1147 = icmp slt i64 %1143, %1146
  br i1 %1147, label %1148, label %1152

1148:                                             ; preds = %1145
  %1149 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1146, i64 %1143 monotonic monotonic, align 8, !noalias !15857
  %1150 = extractvalue { i64, i1 } %1149, 1
  %1151 = extractvalue { i64, i1 } %1149, 0
  br i1 %1150, label %1152, label %1145

1152:                                             ; preds = %1148, %1145
  %1153 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15857
  br label %__rustc::__rust_dealloc (.exit125)

__rustc::__rust_dealloc (.exit125): ; preds = %1134, %1152
  call void @free(ptr noundef nonnull %411) #92, !noalias !15857
  br label %1155

1154:                                             ; preds = %641
  unreachable

1155:                                             ; preds = %__rustc::__rust_dealloc (.exit125), %.loopexit151
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<(u64, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(32) %35)
          to label %1156 unwind label %542

1156:                                             ; preds = %1155
  call void @llvm.lifetime.end.p0(ptr nonnull %35)
  %1157 = icmp eq i64 %404, 0
  br i1 %1157, label %1186, label %1158

1158:                                             ; preds = %1156
  %1159 = mul nuw i64 %404, 24
  call void @llvm.assume(i1 true) [ "nonnull"(ptr %405) ]
  %1160 = load i64, ptr %352, align 8, !noundef !1740
  %1161 = call i64 @llvm.umin.i64(i64 %1159, i64 9223372036854775807)
  %1162 = call i64 @llvm.ssub.sat.i64(i64 %1160, i64 %1161)
  store i64 %1162, ptr %352, align 8
  %1163 = load i64, ptr %353, align 8, !noundef !1740
  %1164 = icmp slt i64 %1162, %1163
  br i1 %1164, label %1165, label %.preheader275

1165:                                             ; preds = %1158
  store i64 %1162, ptr %353, align 8
  br label %.preheader275

.preheader275:                                    ; preds = %1165, %1158
  br label %1166

1166:                                             ; preds = %.preheader275, %1169
  %1167 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8
  %1168 = icmp slt i64 %1167, 0
  br i1 %1168, label %1169, label %__rustc::__rust_dealloc (.exit126)

1169:                                             ; preds = %1166
  %1170 = add nsw i64 %1167, 1
  %1171 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1167, i64 %1170 acq_rel acquire, align 8
  %1172 = extractvalue { i64, i1 } %1171, 1
  br i1 %1172, label %1173, label %1166

1173:                                             ; preds = %1169
  %1174 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1161 monotonic, align 8
  %1175 = call i64 @llvm.ssub.sat.i64(i64 %1174, i64 %1161)
  %1176 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8
  br label %1177

1177:                                             ; preds = %1180, %1173
  %1178 = phi i64 [ %1176, %1173 ], [ %1183, %1180 ]
  %1179 = icmp slt i64 %1175, %1178
  br i1 %1179, label %1180, label %1184

1180:                                             ; preds = %1177
  %1181 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1178, i64 %1175 monotonic monotonic, align 8
  %1182 = extractvalue { i64, i1 } %1181, 1
  %1183 = extractvalue { i64, i1 } %1181, 0
  br i1 %1182, label %1184, label %1177

1184:                                             ; preds = %1180, %1177
  %1185 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8
  br label %__rustc::__rust_dealloc (.exit126)

__rustc::__rust_dealloc (.exit126): ; preds = %1166, %1184
  call void @free(ptr noundef nonnull %405) #92
  br label %1186

1186:                                             ; preds = %__rustc::__rust_dealloc (.exit126), %1156
  call void @llvm.experimental.noalias.scope.decl(metadata !15860)
  %1187 = load ptr, ptr %344, align 8, !alias.scope !15860, !noundef !1740
  %1188 = icmp eq ptr %1187, null
  br i1 %1188, label %1193, label %1189

1189:                                             ; preds = %1186
  %1190 = atomicrmw sub ptr %1187, i64 1 release, align 8, !noalias !15863
  %1191 = icmp eq i64 %1190, 1
  br i1 %1191, label %1192, label %1193

1192:                                             ; preds = %1189
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ExactDeferral>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %344) #91
  br label %1193

1193:                                             ; preds = %1192, %1189, %1186
  call void @llvm.experimental.noalias.scope.decl(metadata !15868)
  %1194 = load ptr, ptr %345, align 8, !alias.scope !15868, !noundef !1740
  %1195 = icmp eq ptr %1194, null
  br i1 %1195, label %1200, label %1196

1196:                                             ; preds = %1193
  %1197 = atomicrmw sub ptr %1194, i64 1 release, align 8, !noalias !15871
  %1198 = icmp eq i64 %1197, 1
  br i1 %1198, label %1199, label %1200

1199:                                             ; preds = %1196
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %345) #91
  br label %1200

1200:                                             ; preds = %1199, %1196, %1193
  call void @llvm.lifetime.end.p0(ptr nonnull %36)
; invoke core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(32) %37)
          to label %1201 unwind label %.loopexit.split-lp

1201:                                             ; preds = %1200
  call void @llvm.lifetime.end.p0(ptr nonnull %37)
  %1202 = atomicrmw sub ptr %276, i64 1 release, align 8, !noalias !15876
  %1203 = icmp eq i64 %1202, 1
  br i1 %1203, label %1204, label %1205

1204:                                             ; preds = %1201
  fence acquire
; invoke <alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow
  invoke void @<alloc::sync::Arc<purrdf_sparql_eval::governor::GovernorState>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %38) #91
          to label %1205 unwind label %249

1205:                                             ; preds = %1204, %1201
  call void @llvm.lifetime.end.p0(ptr nonnull %38)
  call void @llvm.experimental.noalias.scope.decl(metadata !15881)
  call void @llvm.experimental.noalias.scope.decl(metadata !15884)
  %1206 = load ptr, ptr %142, align 8, !alias.scope !15887, !nonnull !1740, !noundef !1740
  %1207 = load ptr, ptr %143, align 8, !alias.scope !15887, !nonnull !1740, !noundef !1740
  %1208 = ptrtoint ptr %1207 to i64
  %1209 = ptrtoint ptr %1206 to i64
  %1210 = sub nuw i64 %1208, %1209
  %1211 = udiv exact i64 %1210, 40
  call void @llvm.experimental.noalias.scope.decl(metadata !15888)
  %1212 = icmp eq ptr %1207, %1206
  br i1 %1212, label %.loopexit141, label %.preheader140

1213:                                             ; preds = %__rustc::__rust_dealloc (.exit128), %.loopexit141
  call void @llvm.lifetime.end.p0(ptr nonnull %41)
  call void @llvm.lifetime.end.p0(ptr nonnull %45)
  br label %454

.preheader140:                                    ; preds = %1205, %1251
  %1214 = phi i64 [ %1216, %1251 ], [ 0, %1205 ]
  %1215 = getelementptr inbounds nuw [40 x i8], ptr %1206, i64 %1214
  %1216 = add nuw nsw i64 %1214, 1
  %1217 = load i64, ptr %1215, align 8, !range !1778, !alias.scope !15891, !noalias !15887, !noundef !1740
  %1218 = icmp ugt i64 %1217, 5
  br i1 %1218, label %1219, label %1251

1219:                                             ; preds = %.preheader140
  %1220 = getelementptr i8, ptr %1215, i64 8
  %1221 = load ptr, ptr %1220, align 8, !alias.scope !15888, !noalias !15887, !nonnull !1740, !noundef !1740
  %1222 = shl i64 %1217, 3
  %1223 = add i64 %1222, -8
  %1224 = load i64, ptr %352, align 8, !noalias !15894, !noundef !1740
  %1225 = call i64 @llvm.umin.i64(i64 %1223, i64 9223372036854775807)
  %1226 = call i64 @llvm.ssub.sat.i64(i64 %1224, i64 %1225)
  store i64 %1226, ptr %352, align 8, !noalias !15894
  %1227 = load i64, ptr %353, align 8, !noalias !15894, !noundef !1740
  %1228 = icmp slt i64 %1226, %1227
  br i1 %1228, label %1229, label %.preheader274

1229:                                             ; preds = %1219
  store i64 %1226, ptr %353, align 8, !noalias !15894
  br label %.preheader274

.preheader274:                                    ; preds = %1229, %1219
  br label %1230

1230:                                             ; preds = %.preheader274, %1233
  %1231 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15894
  %1232 = icmp slt i64 %1231, 0
  br i1 %1232, label %1233, label %__rustc::__rust_dealloc (.exit127)

1233:                                             ; preds = %1230
  %1234 = add nsw i64 %1231, 1
  %1235 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1231, i64 %1234 acq_rel acquire, align 8, !noalias !15894
  %1236 = extractvalue { i64, i1 } %1235, 1
  br i1 %1236, label %1237, label %1230

1237:                                             ; preds = %1233
  %1238 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1225 monotonic, align 8, !noalias !15894
  %1239 = call i64 @llvm.ssub.sat.i64(i64 %1238, i64 %1225)
  %1240 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15894
  br label %1241

1241:                                             ; preds = %1244, %1237
  %1242 = phi i64 [ %1240, %1237 ], [ %1247, %1244 ]
  %1243 = icmp slt i64 %1239, %1242
  br i1 %1243, label %1244, label %1248

1244:                                             ; preds = %1241
  %1245 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1242, i64 %1239 monotonic monotonic, align 8, !noalias !15894
  %1246 = extractvalue { i64, i1 } %1245, 1
  %1247 = extractvalue { i64, i1 } %1245, 0
  br i1 %1246, label %1248, label %1241

1248:                                             ; preds = %1244, %1241
  %1249 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15894
  br label %__rustc::__rust_dealloc (.exit127)

__rustc::__rust_dealloc (.exit127): ; preds = %1230, %1248
  %1250 = icmp ne i64 %1223, 0
  call void @llvm.assume(i1 %1250), !noalias !15894
  call void @free(ptr noundef nonnull %1221) #92, !noalias !15894
  br label %1251

1251:                                             ; preds = %__rustc::__rust_dealloc (.exit127), %.preheader140
  %1252 = icmp eq i64 %1216, %1211
  br i1 %1252, label %.loopexit141, label %.preheader140

.loopexit141:                                     ; preds = %1251, %1205
  %1253 = load i64, ptr %141, align 8, !alias.scope !15887, !noundef !1740
  %1254 = icmp eq i64 %1253, 0
  br i1 %1254, label %1213, label %1255

1255:                                             ; preds = %.loopexit141
  %1256 = load ptr, ptr %41, align 8, !alias.scope !15887, !nonnull !1740, !noundef !1740
  %1257 = mul nuw i64 %1253, 40
  %1258 = load i64, ptr %352, align 8, !noalias !15887, !noundef !1740
  %1259 = call i64 @llvm.umin.i64(i64 %1257, i64 9223372036854775807)
  %1260 = call i64 @llvm.ssub.sat.i64(i64 %1258, i64 %1259)
  store i64 %1260, ptr %352, align 8, !noalias !15887
  %1261 = load i64, ptr %353, align 8, !noalias !15887, !noundef !1740
  %1262 = icmp slt i64 %1260, %1261
  br i1 %1262, label %1263, label %.preheader273

1263:                                             ; preds = %1255
  store i64 %1260, ptr %353, align 8, !noalias !15887
  br label %.preheader273

.preheader273:                                    ; preds = %1263, %1255
  br label %1264

1264:                                             ; preds = %.preheader273, %1267
  %1265 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601) acquire, align 8, !noalias !15887
  %1266 = icmp slt i64 %1265, 0
  br i1 %1266, label %1267, label %__rustc::__rust_dealloc (.exit128)

1267:                                             ; preds = %1264
  %1268 = add nsw i64 %1265, 1
  %1269 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 %1265, i64 %1268 acq_rel acquire, align 8, !noalias !15887
  %1270 = extractvalue { i64, i1 } %1269, 1
  br i1 %1270, label %1271, label %1264

1271:                                             ; preds = %1267
  %1272 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_LIVE_BYTES, i64 %1259 monotonic, align 8, !noalias !15887
  %1273 = call i64 @llvm.ssub.sat.i64(i64 %1272, i64 %1259)
  %1274 = load atomic i64, ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES monotonic, align 8, !noalias !15887
  br label %1275

1275:                                             ; preds = %1278, %1271
  %1276 = phi i64 [ %1274, %1271 ], [ %1281, %1278 ]
  %1277 = icmp slt i64 %1273, %1276
  br i1 %1277, label %1278, label %1282

1278:                                             ; preds = %1275
  %1279 = cmpxchg weak ptr @purrdf_alloc_probe::PROCESS_TROUGH_BYTES, i64 %1276, i64 %1273 monotonic monotonic, align 8, !noalias !15887
  %1280 = extractvalue { i64, i1 } %1279, 1
  %1281 = extractvalue { i64, i1 } %1279, 0
  br i1 %1280, label %1282, label %1275

1282:                                             ; preds = %1278, %1275
  %1283 = atomicrmw sub ptr @purrdf_alloc_probe::PROCESS_STATE (.llvm.6551227014246703601), i64 1 release, align 8, !noalias !15887
  br label %__rustc::__rust_dealloc (.exit128)

__rustc::__rust_dealloc (.exit128): ; preds = %1264, %1282
  call void @free(ptr noundef nonnull %1256) #92, !noalias !15887
  br label %1213

1284:                                             ; preds = %580, %577, %574
  call void @llvm.experimental.noalias.scope.decl(metadata !15897)
  %1285 = load ptr, ptr %345, align 8, !alias.scope !15897, !noundef !1740
  %1286 = icmp eq ptr %1285, null
  br i1 %1286, label %396, label %1287

1287:                                             ; preds = %1284
  %1288 = atomicrmw sub ptr %1285, i64 1 release, align 8, !noalias !15900
  %1289 = icmp eq i64 %1288, 1
  br i1 %1289, label %1290, label %396

1290:                                             ; preds = %1287
  fence acquire
; call <alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow
  call void @<alloc::sync::Arc<purrdf_sparql_eval::row_checkpoint::ForkShared>>::drop_slow(ptr noalias nofree noundef nonnull readonly align 8 dereferenceable(8) %345) #91
  br label %396

1291:                                             ; preds = %381, %375
; call core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(32) %41) #89
  br i1 %376, label %1292, label %1293

1292:                                             ; preds = %1291
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %45) #89
  br i1 %377, label %1299, label %1297

1293:                                             ; preds = %1291
  br i1 %377, label %1299, label %1297

1294:                                             ; preds = %251, %__rustc::__rust_dealloc (.exit118), %.loopexit168
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %3)
  br label %454

1295:                                             ; preds = %6
  %1296 = landingpad { ptr, i32 }
          cleanup
; call core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>>(ptr noalias nofree noundef align 8 dereferenceable(24) %4) #89
  br label %1299

1297:                                             ; preds = %1299, %1293, %1292, %249
  %1298 = phi { ptr, i32 } [ %1300, %1299 ], [ %378, %1293 ], [ %250, %249 ], [ %378, %1292 ]
  resume { ptr, i32 } %1298

1299:                                             ; preds = %1295, %1293, %1292, %159
  %1300 = phi { ptr, i32 } [ %378, %1293 ], [ %1296, %1295 ], [ %378, %1292 ], [ %261, %159 ]
; invoke core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<purrdf_sparql_eval::row_checkpoint::WorkerLedger>>(ptr noalias nofree noundef align 8 dereferenceable(24) %3) #89
          to label %1297 unwind label %273
}
