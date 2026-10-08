define internal fastcc void @<purrdf_core::small::SmallVec<[core::option::Option<purrdf_sparql_eval::scratch::SolutionTerm>; 4]>>::from_elem(ptr dead_on_unwind noalias nofree noundef nonnull writable writeonly align 8 captures(none) dereferenceable(40) %0, i64 noundef %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !84829 {
  %3 = alloca [40 x i8], align 8
  %4 = alloca [40 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %4)
  call void @llvm.lifetime.start.p0(ptr nonnull %3), !noalias !84830
  store i64 1, ptr %3, align 8, !noalias !84830
  %5 = icmp ugt i64 %1, 4
  %.sroa.gep = getelementptr inbounds nuw i8, ptr %4, i64 8
  %.sroa.gep8 = getelementptr inbounds nuw i8, ptr %3, i64 8
  br i1 %5, label %6, label %20, !prof !1742

6:                                                ; preds = %2
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %3, i64 noundef 0, i64 noundef %1, i1 noundef zeroext false) #92
          to label %7 unwind label %8, !noalias !84830

7:                                                ; preds = %6
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %4, ptr noundef nonnull align 8 dereferenceable(40) %3, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %3), !noalias !84830
  br label %31

8:                                                ; preds = %6
  %9 = landingpad { ptr, i32 }
          cleanup
  %10 = load i64, ptr %3, align 8, !range !1778, !alias.scope !84833, !noalias !84830, !noundef !1740
  %11 = icmp ugt i64 %10, 5
  br i1 %11, label %12, label %18

12:                                               ; preds = %27, %8
  %.sroa.phi = phi ptr [ %.sroa.gep, %27 ], [ %.sroa.gep8, %8 ]
  %13 = phi i64 [ %29, %27 ], [ %10, %8 ]
  %14 = phi { ptr, i32 } [ %28, %27 ], [ %9, %8 ]
  %15 = load ptr, ptr %.sroa.phi, align 8, !nonnull !1740, !noundef !1740
  %16 = shl i64 %13, 3
  %17 = add i64 %16, -8
; call __rustc::__rust_dealloc
  call void @__rustc::__rust_dealloc(ptr noundef nonnull %15, i64 noundef %17, i64 noundef range(i64 1, -9223372036854775807) 4) #93, !noalias !1740
  br label %18

18:                                               ; preds = %27, %12, %8
  %19 = phi { ptr, i32 } [ %9, %8 ], [ %28, %27 ], [ %14, %12 ]
  resume { ptr, i32 } %19

20:                                               ; preds = %2
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %4, ptr noundef nonnull align 8 dereferenceable(40) %3, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %3), !noalias !84830
  %21 = icmp eq i64 %1, 0
  %22 = select i1 %21, i32 -1, i32 2
  br label %31

23:                                               ; preds = %90
  %24 = landingpad { ptr, i32 }
          cleanup
  br label %27

25:                                               ; preds = %43
  %26 = landingpad { ptr, i32 }
          cleanup
  br label %27

27:                                               ; preds = %25, %23
  %28 = phi { ptr, i32 } [ %24, %23 ], [ %26, %25 ]
  %29 = load i64, ptr %4, align 8, !range !1778, !alias.scope !18581, !noundef !1740
  %30 = icmp ugt i64 %29, 5
  br i1 %30, label %12, label %18

31:                                               ; preds = %20, %7
  %32 = phi i32 [ 2, %7 ], [ %22, %20 ]
  %33 = load i64, ptr %4, align 8, !range !1778, !alias.scope !84836, !noundef !1740
  %34 = add i64 %33, -1
  %35 = icmp ugt i64 %34, 4
  %36 = getelementptr inbounds nuw i8, ptr %4, i64 16
  %37 = load i64, ptr %36, align 8, !alias.scope !84836
  %38 = add i64 %37, -1
  %39 = select i1 %35, i64 %38, i64 %34
  %40 = call i64 @llvm.umax.i64(i64 %34, i64 4)
  %41 = sub i64 %40, %39
  %42 = icmp ult i64 %41, %1
  br i1 %42, label %43, label %48, !prof !1742

43:                                               ; preds = %31
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %4, i64 noundef %39, i64 noundef %1, i1 noundef zeroext true) #92
          to label %44 unwind label %25

44:                                               ; preds = %43
  %45 = load i64, ptr %4, align 8, !range !1778, !alias.scope !84839, !noalias !84842
  %46 = add i64 %45, -1
  %47 = call i64 @llvm.umax.i64(i64 %46, i64 4)
  br label %48

48:                                               ; preds = %44, %31
  %49 = phi i64 [ %47, %44 ], [ %40, %31 ]
  %50 = phi i64 [ %46, %44 ], [ %34, %31 ]
  %51 = icmp ugt i64 %50, 4
  %52 = load ptr, ptr %.sroa.gep, align 8, !alias.scope !84839, !noalias !84842, !nonnull !1740
  %53 = select i1 %51, ptr %52, ptr %.sroa.gep
  %54 = select i1 %51, ptr %36, ptr %4
  %55 = load i64, ptr %54, align 8, !alias.scope !84839, !noalias !84842, !noundef !1740
  %56 = add i64 %55, -1
  %57 = icmp ult i64 %56, %49
  br i1 %57, label %.preheader9, label %60

58:                                               ; preds = %69
  %59 = add nuw i64 %49, 1
  br label %60

60:                                               ; preds = %58, %48
  %61 = phi i32 [ %32, %48 ], [ %72, %58 ]
  %62 = phi i64 [ %1, %48 ], [ %73, %58 ]
  %63 = phi i64 [ %55, %48 ], [ %59, %58 ]
  store i64 %63, ptr %54, align 8
  %64 = icmp eq i32 %61, -1
  br i1 %64, label %.loopexit, label %.preheader

.preheader9:                                      ; preds = %48, %69
  %65 = phi i64 [ %75, %69 ], [ %56, %48 ]
  %66 = phi i64 [ %73, %69 ], [ %1, %48 ]
  %67 = phi i32 [ %72, %69 ], [ %32, %48 ]
  %68 = icmp eq i32 %67, -1
  br i1 %68, label %102, label %69

69:                                               ; preds = %.preheader9
  %70 = add i64 %66, -1
  %71 = icmp eq i64 %70, 0
  %72 = select i1 %71, i32 -1, i32 %67
  %73 = call i64 @llvm.umax.i64(i64 %70, i64 1)
  %74 = getelementptr inbounds nuw [8 x i8], ptr %53, i64 %65
  store i32 %67, ptr %74, align 4, !noalias !84842
  %75 = add i64 %65, 1
  %76 = icmp eq i64 %75, %49
  br i1 %76, label %58, label %.preheader9

.preheader:                                       ; preds = %60, %97
  %77 = phi i64 [ %78, %97 ], [ %62, %60 ]
  %78 = add i64 %77, -1
  %79 = icmp eq i64 %78, 0
  %80 = load i64, ptr %4, align 8, !range !1778, !alias.scope !84844, !noundef !1740
  %81 = add i64 %80, -1
  %82 = icmp ugt i64 %81, 4
  %83 = load ptr, ptr %.sroa.gep, align 8, !alias.scope !84844, !nonnull !1740
  %84 = select i1 %82, ptr %83, ptr %.sroa.gep
  %85 = select i1 %82, ptr %36, ptr %4
  %86 = call i64 @llvm.umax.i64(i64 %81, i64 4)
  %87 = load i64, ptr %85, align 8, !alias.scope !84844, !noundef !1740
  %88 = add i64 %87, -1
  %89 = icmp eq i64 %88, %86
  br i1 %89, label %90, label %97, !prof !1742

90:                                               ; preds = %.preheader
; invoke <purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow
  invoke void @<purrdf_core::small::SmallVec<[purrdf_sparql_eval::vm::ConstCell<purrdf_core::ir::term::TermId>; 4]>>::grow(ptr noalias nofree noundef nonnull align 8 dereferenceable(40) %4, i64 noundef %86, i64 noundef 1, i1 noundef zeroext true) #92
          to label %91 unwind label %23

91:                                               ; preds = %90
  %92 = load i64, ptr %4, align 8, !range !1778, !alias.scope !84844, !noundef !1740
  %93 = icmp ugt i64 %92, 5
  %94 = load ptr, ptr %.sroa.gep, align 8, !alias.scope !84844, !nonnull !1740
  %95 = select i1 %93, ptr %94, ptr %.sroa.gep
  %96 = select i1 %93, ptr %36, ptr %4
  br label %97

97:                                               ; preds = %91, %.preheader
  %98 = phi ptr [ %95, %91 ], [ %84, %.preheader ]
  %99 = phi ptr [ %96, %91 ], [ %85, %.preheader ]
  %100 = getelementptr inbounds nuw [8 x i8], ptr %98, i64 %88
  store i32 %61, ptr %100, align 4
  %101 = add i64 %87, 1
  store i64 %101, ptr %99, align 8, !alias.scope !84844
  br i1 %79, label %.loopexit, label %.preheader

102:                                              ; preds = %.preheader9
  %103 = add nuw i64 %65, 1
  store i64 %103, ptr %54, align 8
  br label %.loopexit

.loopexit:                                        ; preds = %97, %102, %60
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(40) %0, ptr noundef nonnull align 8 dereferenceable(40) %4, i64 40, i1 false)
  call void @llvm.lifetime.end.p0(ptr nonnull %4)
  ret void
}
