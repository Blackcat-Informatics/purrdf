define internal fastcc void @core::ptr::drop_glue::<<purrdf_native::py_store::quad_store::PyQuadStore>::query_governed::{closure#0}::{closure#0}>(ptr noalias nofree noundef nonnull readonly align 8 captures(none) dereferenceable(272) %_1) unnamed_addr #1 personality ptr @rust_eh_personality !guid !27804 {
start:
; invoke core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>
  invoke fastcc void @core::ptr::drop_glue::<alloc::vec::Vec<(alloc::string::String, purrdf_native::py_store::query::RelationSpec, purrdf_native::attestation::Attestation)>>(ptr noalias nofree noundef align 8 dereferenceable(24) %_1)
          to label %bb8 unwind label %cleanup

cleanup:                                          ; preds = %start
  %0 = landingpad { ptr, i32 }
          cleanup
  %1 = getelementptr inbounds nuw i8, ptr %_1, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !27805)
  %2 = load i64, ptr %1, align 8, !range !3909, !alias.scope !27805, !noundef !3892
  %3 = icmp eq i64 %2, -1
  br i1 %3, label %bb4, label %bb2.i

bb2.i:                                            ; preds = %cleanup
  tail call void @llvm.experimental.noalias.scope.decl(metadata !27808)
  %4 = icmp eq i64 %2, 0
  br i1 %4, label %bb4, label %bb2.i.i.i4.i.i.i

bb2.i.i.i4.i.i.i:                                 ; preds = %bb2.i
  %5 = getelementptr inbounds nuw i8, ptr %_1, i64 56
  %_1.val1.i.i = load ptr, ptr %5, align 8, !alias.scope !27811, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i, i64 noundef %2, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !27811
  br label %bb4

bb8:                                              ; preds = %start
  %6 = getelementptr inbounds nuw i8, ptr %_1, i64 48
  tail call void @llvm.experimental.noalias.scope.decl(metadata !27812)
  %7 = load i64, ptr %6, align 8, !range !3909, !alias.scope !27812, !noundef !3892
  %8 = icmp eq i64 %7, -1
  br i1 %8, label %bb7, label %bb2.i6

bb2.i6:                                           ; preds = %bb8
  tail call void @llvm.experimental.noalias.scope.decl(metadata !27815)
  %9 = icmp eq i64 %7, 0
  br i1 %9, label %bb7, label %bb2.i.i.i4.i.i.i7

bb2.i.i.i4.i.i.i7:                                ; preds = %bb2.i6
  %10 = getelementptr inbounds nuw i8, ptr %_1, i64 56
  %_1.val1.i.i8 = load ptr, ptr %10, align 8, !alias.scope !27818, !nonnull !3892, !noundef !3892
; call __rustc::__rust_dealloc
  tail call void @__rustc::__rust_dealloc(ptr noundef nonnull %_1.val1.i.i8, i64 noundef %7, i64 noundef range(i64 1, -9223372036854775807) 1) #77, !noalias !27818
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
