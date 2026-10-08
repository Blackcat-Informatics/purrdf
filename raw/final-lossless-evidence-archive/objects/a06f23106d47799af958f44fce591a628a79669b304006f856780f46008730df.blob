define internal fastcc void @core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<(bool, core::option::Option<&pyo3::instance::Bound<pyo3::types::any::PyAny>>, bool)>::{closure#0}::{closure#0}>(ptr noalias nofree noundef nonnull align 8 captures(none) dereferenceable(360) %_1) unnamed_addr #1 personality ptr @rust_eh_personality !guid !30593 {
start:
  %0 = getelementptr inbounds nuw i8, ptr %_1, i64 240
  %1 = load i64, ptr %0, align 8, !range !4141, !alias.scope !30594, !noundef !3995
  %2 = icmp eq i64 %1, -1
  br i1 %2, label %bb10, label %bb2.i.i

bb2.i.i:                                          ; preds = %start
; invoke core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>
  invoke fastcc void @core::ptr::drop_glue::<purrdf_core::ir::term::TermValue>(ptr noalias nofree noundef nonnull align 8 dereferenceable(88) %0)
          to label %bb10 unwind label %cleanup

cleanup:                                          ; preds = %bb2.i.i
  %3 = landingpad { ptr, i32 }
          cleanup
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_1) #79
          to label %bb5 unwind label %terminate

bb10:                                             ; preds = %start, %bb2.i.i
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_1)
          to label %bb9 unwind label %cleanup1

bb5:                                              ; preds = %cleanup, %cleanup1
  %.pn = phi { ptr, i32 } [ %9, %cleanup1 ], [ %3, %cleanup ]
  %4 = getelementptr inbounds nuw i8, ptr %_1, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30599)
  %5 = load i64, ptr %4, align 8, !range !4012, !alias.scope !30599, !noundef !3995
  %6 = icmp eq i64 %5, -1
  br i1 %6, label %bb4, label %bb2.i

bb2.i:                                            ; preds = %bb5
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30602)
  %7 = icmp eq i64 %5, 0
  br i1 %7, label %bb4, label %bb2.i.i.i4.i.i.i

bb2.i.i.i4.i.i.i:                                 ; preds = %bb2.i
  %8 = getelementptr inbounds nuw i8, ptr %_1, i64 56
  %_1.val1.i.i = load ptr, ptr %8, align 8, !alias.scope !30605, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i, i64 noundef %5, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !30605
  br label %bb4

cleanup1:                                         ; preds = %bb10
  %9 = landingpad { ptr, i32 }
          cleanup
  br label %bb5

bb9:                                              ; preds = %bb10
  %10 = getelementptr inbounds nuw i8, ptr %_1, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30606)
  %11 = load i64, ptr %10, align 8, !range !4012, !alias.scope !30606, !noundef !3995
  %12 = icmp eq i64 %11, -1
  br i1 %12, label %bb8, label %bb2.i9

bb2.i9:                                           ; preds = %bb9
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30609)
  %13 = icmp eq i64 %11, 0
  br i1 %13, label %bb8, label %bb2.i.i.i4.i.i.i10

bb2.i.i.i4.i.i.i10:                               ; preds = %bb2.i9
  %14 = getelementptr inbounds nuw i8, ptr %_1, i64 56
  %_1.val1.i.i11 = load ptr, ptr %14, align 8, !alias.scope !30612, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i11, i64 noundef %11, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !30612
  br label %bb8

bb4:                                              ; preds = %bb2.i.i.i4.i.i.i, %bb2.i, %bb5
  %15 = getelementptr inbounds nuw i8, ptr %_1, i64 72
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  tail call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef align 8 dereferenceable(168) %15) #79
  %16 = getelementptr inbounds nuw i8, ptr %_1, i64 24
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %16) #79
          to label %bb2 unwind label %terminate

bb8:                                              ; preds = %bb2.i.i.i4.i.i.i10, %bb2.i9, %bb9
  %17 = getelementptr inbounds nuw i8, ptr %_1, i64 72
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  tail call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef align 8 dereferenceable(168) %17)
  %18 = getelementptr inbounds nuw i8, ptr %_1, i64 24
; call core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %18)
  ret void

terminate:                                        ; preds = %bb4, %cleanup
  %19 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #75
  unreachable

bb2:                                              ; preds = %bb4
  resume { ptr, i32 } %.pn
}
define internal fastcc void @core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_impl<()>::{closure#0}::{closure#0}>(ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(272) %_1) unnamed_addr #1 personality ptr @rust_eh_personality !guid !30613 {
start:
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_1)
          to label %bb8 unwind label %cleanup

cleanup:                                          ; preds = %start
  %0 = landingpad { ptr, i32 }
          cleanup
  %1 = getelementptr inbounds nuw i8, ptr %_1, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30614)
  %2 = load i64, ptr %1, align 8, !range !4012, !alias.scope !30614, !noundef !3995
  %3 = icmp eq i64 %2, -1
  br i1 %3, label %bb4, label %bb2.i

bb2.i:                                            ; preds = %cleanup
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30617)
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %bb4, label %bb2.i.i.i4.i.i.i

bb2.i.i.i4.i.i.i:                                 ; preds = %bb2.i
  %5 = getelementptr inbounds nuw i8, ptr %_1, i64 56
  %_1.val1.i.i = load ptr, ptr %5, align 8, !alias.scope !30620, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i, i64 noundef %2, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !30620
  br label %bb4

bb8:                                              ; preds = %start
  %6 = getelementptr inbounds nuw i8, ptr %_1, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30621)
  %7 = load i64, ptr %6, align 8, !range !4012, !alias.scope !30621, !noundef !3995
  %8 = icmp eq i64 %7, -1
  br i1 %8, label %bb7, label %bb2.i6

bb2.i6:                                           ; preds = %bb8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !30624)
  %9 = icmp eq i64 %7, 0
  br i1 %9, label %bb7, label %bb2.i.i.i4.i.i.i7

bb2.i.i.i4.i.i.i7:                                ; preds = %bb2.i6
  %10 = getelementptr inbounds nuw i8, ptr %_1, i64 56
  %_1.val1.i.i8 = load ptr, ptr %10, align 8, !alias.scope !30627, !nonnull !3995, !noundef !3995
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i8, i64 noundef %7, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !30627
  br label %bb7

bb4:                                              ; preds = %bb2.i.i.i4.i.i.i, %bb2.i, %cleanup
  %11 = getelementptr inbounds nuw i8, ptr %_1, i64 72
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  tail call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef align 8 dereferenceable(168) %11) #79
  %12 = getelementptr inbounds nuw i8, ptr %_1, i64 24
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %12) #79
          to label %bb2 unwind label %terminate

bb7:                                              ; preds = %bb2.i.i.i4.i.i.i7, %bb2.i6, %bb8
  %13 = getelementptr inbounds nuw i8, ptr %_1, i64 72
; call core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>
  tail call fastcc void @core::ptr::drop_glue::<purrdf_native::py_store::query::EngineConfig>(ptr noalias nofree noundef align 8 dereferenceable(168) %13)
  %14 = getelementptr inbounds nuw i8, ptr %_1, i64 24
; call core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>
  tail call fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_core::ir::term::TermValue)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %14)
  ret void

terminate:                                        ; preds = %bb4
  %15 = landingpad { ptr, i32 }
          filter [0 x ptr] zeroinitializer
; call core::panicking::panic_in_cleanup
  tail call void @core::panicking::panic_in_cleanup() #75
  unreachable

bb2:                                              ; preds = %bb4
  resume { ptr, i32 } %0
}
