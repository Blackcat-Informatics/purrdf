define internal fastcc noundef zeroext i1 @purrdf_sparql_eval::modifier::graph_holds_no_rows::<purrdf_core::ir::dataset::RdfDataset>(ptr noundef nonnull align 8 %0, i32 noundef range(i32 1, 0) %1) unnamed_addr #0 personality ptr @rust_eh_personality !guid !41371 {
  %3 = alloca [16 x i8], align 8
  %4 = alloca [8 x i8], align 8
  %5 = alloca [16 x i8], align 8
  %6 = alloca [72 x i8], align 8
  %7 = alloca [16 x i8], align 4
  %8 = alloca [88 x i8], align 8
  call void @llvm.lifetime.start.p0(ptr nonnull %8)
  call void @llvm.lifetime.start.p0(ptr nonnull %5), !noalias !41372
; call <purrdf_core::ir::dataset::RdfDataset>::probe_plan
  %9 = tail call { i64, i8 } @<purrdf_core::ir::dataset::RdfDataset>::probe_plan(i1 noundef zeroext false, i1 noundef zeroext false, i1 noundef zeroext false, i32 noundef 2, i32 poison)
  %10 = extractvalue { i64, i8 } %9, 0
  %11 = extractvalue { i64, i8 } %9, 1
  store i64 %10, ptr %5, align 8, !noalias !41372
  %12 = getelementptr inbounds nuw i8, ptr %5, i64 8
  store i8 %11, ptr %12, align 8, !noalias !41372
; call <purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan
  call void @<purrdf_core::ir::dataset::RdfDataset>::quads_for_pattern_with_plan(ptr noalias nofree noundef nonnull sret([88 x i8]) align 8 captures(none) dereferenceable(88) %8, ptr noundef nonnull align 8 %0, ptr noalias nofree noundef nonnull readonly align 8 captures(address, read_provenance) dereferenceable(16) %5, i32 noundef 0, i32 noundef 0, i32 noundef 0, i32 noundef 2, i32 %1)
  call void @llvm.lifetime.end.p0(ptr nonnull %5), !noalias !41372
; call <purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next
  %13 = call fastcc noundef align 4 ptr @<purrdf_core::ir::dataset::QuadMatches as core::iter::traits::iterator::Iterator>::next(ptr noalias nofree noundef nonnull align 8 dereferenceable(88) %8) #87, !noalias !41375
  %14 = icmp eq ptr %13, null
  call void @llvm.lifetime.end.p0(ptr nonnull %8)
  br i1 %14, label %15, label %.loopexit

15:                                               ; preds = %2
  call void @llvm.lifetime.start.p0(ptr nonnull %7)
  call void @llvm.lifetime.start.p0(ptr nonnull %6)
  %16 = getelementptr inbounds nuw i8, ptr %6, i64 8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !41378)
  %17 = getelementptr inbounds nuw i8, ptr %0, i64 80
  %18 = load i64, ptr %17, align 8, !noalias !41378, !noundef !1733
  %19 = icmp eq i64 %18, 0
  br i1 %19, label %<purrdf_core::ir::dataset::RdfDataset>::reifier_quads (.exit), label %20

20:                                               ; preds = %15
; call <purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri
  %21 = tail call noundef i32 @<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri(ptr noundef nonnull readonly align 8 %0, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158, i64 noundef 50), !noalias !41378
  %22 = icmp eq i32 %21, 0
  br i1 %22, label %23, label %<purrdf_core::ir::dataset::RdfDataset>::reifier_quads (.exit), !prof !1735

23:                                               ; preds = %20
  call void @llvm.lifetime.start.p0(ptr nonnull %4), !noalias !41378
  %24 = load i64, ptr %17, align 8, !noalias !41378, !noundef !1733
  store i64 %24, ptr %4, align 8, !noalias !41378
  call void @llvm.lifetime.start.p0(ptr nonnull %3), !noalias !41378
  store ptr %4, ptr %3, align 8, !noalias !41378
  %25 = getelementptr inbounds nuw i8, ptr %3, i64 8
  store ptr @<usize as core::fmt::Display>::fmt, ptr %25, align 8, !noalias !41378
; call core::panicking::panic_fmt
  call void @core::panicking::panic_fmt(ptr noundef nonnull @anon.4c9120963acf3a9d820f038c36ebab01.4165.llvm.6298868053391388158, ptr noundef nonnull %3, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(24) @anon.4c9120963acf3a9d820f038c36ebab01.4166.llvm.6298868053391388158) #88, !noalias !41378
  unreachable

<purrdf_core::ir::dataset::RdfDataset>::reifier_quads (.exit): ; preds = %15, %20
; call <purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri
  %26 = tail call noundef i32 @<purrdf_core::ir::dataset::RdfDataset>::term_id_by_iri(ptr noundef nonnull readonly align 8 %0, ptr noalias nofree noundef nonnull readonly captures(address, read_provenance) @anon.4c9120963acf3a9d820f038c36ebab01.235.llvm.6298868053391388158, i64 noundef 50), !noalias !41378
  store ptr %0, ptr %16, align 8, !alias.scope !41378
  %27 = getelementptr inbounds nuw i8, ptr %6, i64 16
  store i32 %26, ptr %27, align 8, !alias.scope !41378
  %28 = getelementptr inbounds nuw i8, ptr %6, i64 24
  store ptr null, ptr %28, align 8, !alias.scope !41378
  %29 = getelementptr inbounds nuw i8, ptr %6, i64 48
  store ptr null, ptr %29, align 8, !alias.scope !41378
  store i32 2, ptr %6, align 8, !alias.scope !41381, !noalias !41386
  %30 = getelementptr inbounds nuw i8, ptr %6, i64 4
  store i32 %1, ptr %30, align 4, !alias.scope !41381, !noalias !41386
; call <core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next
  call fastcc void @<core::iter::adapters::filter::Filter<core::iter::adapters::flatten::FlatMap<core::option::IntoIter<purrdf_core::ir::term::TermId>, core::iter::adapters::map::Map<core::iter::adapters::copied::Copied<core::slice::iter::Iter<(purrdf_core::ir::term::TermId, purrdf_core::ir::term::TermId, core::option::Option<purrdf_core::ir::term::TermId>)>>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset>::reifier_quads::{closure#0}>, <purrdf_core::ir::dataset::RdfDataset as purrdf_core::dataset_view::DatasetView>::reifier_quads_in_graph::{closure#0}> as core::iter::traits::iterator::Iterator>::next(ptr noalias nofree noundef align 4 captures(address) dereferenceable(16) %7, ptr noalias nofree noundef align 8 dereferenceable(72) %6) #87
  %31 = load i32, ptr %7, align 4, !noundef !1733
  %32 = icmp eq i32 %31, 0
  call void @llvm.lifetime.end.p0(ptr nonnull %6)
  call void @llvm.lifetime.end.p0(ptr nonnull %7)
  br i1 %32, label %33, label %.loopexit

33:                                               ; preds = %<purrdf_core::ir::dataset::RdfDataset>::reifier_quads (.exit)
  %34 = getelementptr i8, ptr %0, i64 88
  %35 = load ptr, ptr %34, align 8, !nonnull !1733, !noundef !1733
  %36 = getelementptr i8, ptr %0, i64 96
  %37 = load i64, ptr %36, align 8, !noundef !1733
  %38 = shl nuw nsw i64 %37, 4
  %39 = getelementptr inbounds nuw i8, ptr %35, i64 %38
  %40 = icmp eq i64 %37, 0
  br i1 %40, label %.loopexit, label %.preheader

.preheader:                                       ; preds = %33, %51
  %41 = phi ptr [ %42, %51 ], [ %35, %33 ]
  %42 = getelementptr inbounds nuw i8, ptr %41, i64 16
  %43 = getelementptr inbounds nuw i8, ptr %41, i64 12
  %44 = load i32, ptr %43, align 4, !alias.scope !41388, !noalias !41391
  %45 = icmp eq i32 %44, 0
  br i1 %45, label %51, label %46

46:                                               ; preds = %.preheader
  %47 = load i32, ptr %41, align 4, !alias.scope !41388, !noalias !41391
  %48 = icmp ne i32 %44, %1
  %49 = icmp eq i32 %47, 0
  %50 = select i1 %48, i1 true, i1 %49
  br i1 %50, label %51, label %.loopexit

51:                                               ; preds = %46, %.preheader
  %52 = icmp eq ptr %42, %39
  br i1 %52, label %.loopexit, label %.preheader

.loopexit:                                        ; preds = %51, %46, %33, %<purrdf_core::ir::dataset::RdfDataset>::reifier_quads (.exit), %2
  %53 = phi i1 [ false, %<purrdf_core::ir::dataset::RdfDataset>::reifier_quads (.exit) ], [ false, %2 ], [ true, %33 ], [ true, %51 ], [ false, %46 ]
  ret i1 %53
}
