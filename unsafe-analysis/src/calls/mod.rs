use rustc::hir;
use rustc::lint::LateContext;
use rustc::mir;
use rustc::mir::{BasicBlock, Location, Mir, Terminator, TerminatorKind, Safety, ClearCrossCrate,
                 Rvalue, AggregateKind, SourceInfo, SourceScope, SourceScopeLocalData};
use rustc::mir::visit::Visitor;
use rustc_data_structures::indexed_vec::IndexVec;
use rustc::ty::TyKind;
use rustc_target;
use rustc::hir::def_id::DefId;
use convert_abi;
use get_fn_path;

// counts all unsafe class originating from user labeled unsafe (block or function)
// a restricted analysis does not make sense in this case because the unsafe fuction call is
// an unsafe operation itself and then the unsafe labeling is necessary
pub fn run_analysis<'a, 'tcx>(cx: &'a LateContext<'a, 'tcx>) -> Vec<results::calls::ExternalCall> {
    let mut data = Vec::new();
    for &def_id in cx.tcx.mir_keys(hir::def_id::LOCAL_CRATE).iter() {
        let mir = &cx.tcx.optimized_mir(def_id);
        if let Some (mut visitor) = UnsafeCallsVisitor::new(cx, mir, def_id, &mut data) {
            visitor.visit_mir(mir);
        }
    }
    data
}

struct UnsafeCallsVisitor<'a, 'tcx: 'a> {
    cx: &'a LateContext<'a, 'tcx>,
    mir: &'tcx Mir<'tcx>,
    fn_def_id: DefId,
    data: &'a mut Vec<results::calls::ExternalCall>,
    source_info: SourceInfo,
    source_scope_local_data: &'a IndexVec<SourceScope, SourceScopeLocalData>,
}

impl<'a, 'tcx> UnsafeCallsVisitor<'a, 'tcx> {

    fn new(cx: &'a LateContext<'a, 'tcx>, mir: &'tcx Mir, fn_def_id: DefId,
           data: &'a mut Vec<results::calls::ExternalCall>) -> Option<Self> {

        match mir.source_scope_local_data {
            ClearCrossCrate::Set(ref local_data) => Some(UnsafeCallsVisitor {
                cx, mir, fn_def_id, data,
                source_info: SourceInfo {
                    span: mir.span,
                    scope: mir::OUTERMOST_SOURCE_SCOPE,
                },
                source_scope_local_data: local_data,
            }),
            ClearCrossCrate::Clear => {
                info!("unsafety_violations: {:?} - remote, skipping", fn_def_id);
                None
            }
        }
    }

    fn is_user_unsafety(&self) -> bool {
        match self.source_scope_local_data[self.source_info.scope].safety {
            Safety::Safe => false,
            Safety::BuiltinUnsafe => {
                false
            }
            Safety::FnUnsafe => true,
            Safety::ExplicitUnsafe(node_id) => {
                let node = self.cx.tcx.hir.get(node_id);
                if let hir::Node::Block(block) = node {
                    match block.rules {
                        hir::BlockCheckMode::DefaultBlock => {
                            assert!(false);
                            false
                        }
                        hir::BlockCheckMode::UnsafeBlock(unsafe_source) |
                        hir::BlockCheckMode::PushUnsafeBlock(unsafe_source) |
                        hir::BlockCheckMode::PopUnsafeBlock(unsafe_source) => {
                            match unsafe_source {
                                hir::UnsafeSource::UserProvided => {
                                    true
                                }
                                hir::UnsafeSource::CompilerGenerated => {
                                    false
                                }
                            }
                        }
                    }
                } else {
                    assert!(false);
                    false
                }
            }
        }
    }

    fn get_external_call(&self, abi: rustc_target::spec::abi::Abi, def_id: DefId) -> results::calls::ExternalCall {
        let crate_name =
            if def_id.is_local() {
                ::local_crate_name()
            } else {
                self.cx.tcx.crate_name(def_id.krate).to_string()
            };

        results::calls::ExternalCall {
            abi: convert_abi(abi),
            def_path: get_fn_path( self.cx, def_id),
            name: self.cx.tcx.item_name(def_id).to_string(),
            crate_name,
            user_provided: self.is_user_unsafety(),
        }
    }
}

impl<'a, 'tcx> Visitor<'tcx> for UnsafeCallsVisitor<'a, 'tcx> {

    fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: Location) {
        if let &Rvalue::Aggregate(box ref aggregate, _) = rvalue {
            match aggregate {
                &AggregateKind::Array(..) | &AggregateKind::Tuple | &AggregateKind::Adt(..) => {}
                &AggregateKind::Closure(def_id, _) | &AggregateKind::Generator(def_id, _, _) => {
                    // TODO add tests for this
                    //TODO check why Rust unsafe analysis is on mir_built
                    let mir = &mut self.cx.tcx.optimized_mir(def_id);
                    let mut body_visitor = UnsafeCallsVisitor {
                        cx: self.cx,
                        fn_def_id: self.fn_def_id,
                        mir,
                        data: self.data,
                        source_info: self.source_info,
                        source_scope_local_data: self.source_scope_local_data,
                    };
                    body_visitor.visit_mir(mir);
                }
            }
        }
        self.super_rvalue(rvalue, location);
    }

    fn visit_terminator(
        &mut self,
        _block: BasicBlock,
        terminator: &Terminator<'tcx>,
        _location: Location,
    ) {
        if let TerminatorKind::Call {
            ref func,
            args: _,
            destination: _,
            cleanup: _,
        } = terminator.kind {
                match func.ty(&self.mir.local_decls, self.cx.tcx).sty {
                    TyKind::FnDef(callee_def_id, substs) => {
                        if let hir::Unsafety::Unsafe = self.cx.tcx.fn_sig(callee_def_id).unsafety() {
                            let call = self.get_external_call(self.cx.tcx.fn_sig(callee_def_id).abi(), callee_def_id);
                            self.data.push(call);
                        }
                    }
                    TyKind::FnPtr(ref poly_sig) => {
                        match func {
                            mir::Operand::Move(arg)
                            | mir::Operand::Copy(arg) => {
                                info!("func {:?} is fn ptr", arg.ty(&self.mir.local_decls, self.cx.tcx));
                                if let hir::Unsafety::Unsafe = poly_sig.unsafety() {
                                    let elt = results::calls::ExternalCall {
                                        abi: convert_abi(poly_sig.abi()),
                                        def_path: "Unsafe_Call_Fn_Ptr".to_string(),
                                        name: arg.ty(&self.mir.local_decls, self.cx.tcx).to_ty(self.cx.tcx).to_string(),
                                        crate_name: "Unsafe_Call_Fn_Ptr".to_string(),
                                        user_provided: self.is_user_unsafety()
                                    };
                                    self.data.push(elt);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        error!("TypeVariants NOT handled {:?}", func.ty(&self.mir.local_decls, self.cx.tcx).sty);
                    }
                }
        }
    }
}


