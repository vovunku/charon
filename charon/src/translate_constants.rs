//! Functions to translate constants to LLBC.
#![allow(dead_code)]
use crate::common::*;
use crate::expressions as e;
use crate::get_mir::extract_constants_at_top_level;
use crate::translate_ctx::*;
use crate::types as ty;
use crate::values as v;
use rustc_hir::def_id::DefId;
use rustc_middle::mir;
use rustc_middle::ty as mir_ty;
use rustc_middle::ty::{ConstKind, Ty, TyKind};
use std::iter::zip;

/// Translate a typed constant value (either a bool, a char or an integer).
fn translate_constant_integer_like_value(
    ty: &ty::ETy,
    scalar: &mir::interpret::Scalar,
) -> v::Literal {
    trace!();
    // The documentation explicitly says not to match on a scalar.
    // We match on the type and convert the value following this,
    // by calling the appropriate `to_*` method.
    match ty {
        ty::Ty::Literal(ty::LiteralTy::Bool) => v::Literal::Bool(scalar.to_bool().unwrap()),
        ty::Ty::Literal(ty::LiteralTy::Char) => v::Literal::Char(scalar.to_char().unwrap()),
        ty::Ty::Literal(ty::LiteralTy::Integer(i)) => v::Literal::Scalar(match i {
            ty::IntegerTy::Isize => {
                // This is a bit annoying: there is no
                // `to_isize`. For now, we make the hypothesis
                // that isize is an int64
                assert!(std::mem::size_of::<isize>() == 8);
                v::ScalarValue::Isize(scalar.to_i64().unwrap())
            }
            ty::IntegerTy::Usize => {
                // Same as above for usize.
                assert!(std::mem::size_of::<usize>() == 8);
                v::ScalarValue::Usize(scalar.to_u64().unwrap())
            }
            ty::IntegerTy::I8 => v::ScalarValue::I8(scalar.to_i8().unwrap()),
            ty::IntegerTy::U8 => v::ScalarValue::U8(scalar.to_u8().unwrap()),
            ty::IntegerTy::I16 => v::ScalarValue::I16(scalar.to_i16().unwrap()),
            ty::IntegerTy::U16 => v::ScalarValue::U16(scalar.to_u16().unwrap()),
            ty::IntegerTy::I32 => v::ScalarValue::I32(scalar.to_i32().unwrap()),
            ty::IntegerTy::U32 => v::ScalarValue::U32(scalar.to_u32().unwrap()),
            ty::IntegerTy::I64 => v::ScalarValue::I64(scalar.to_i64().unwrap()),
            ty::IntegerTy::U64 => v::ScalarValue::U64(scalar.to_u64().unwrap()),
            ty::IntegerTy::I128 => v::ScalarValue::I128(scalar.to_i128().unwrap()),
            ty::IntegerTy::U128 => v::ScalarValue::U128(scalar.to_u128().unwrap()),
        }),
        _ => {
            // The remaining types should not be used for constants,
            // or should have been filtered by the caller.
            error!("unexpected type: {:?}", ty);
            unreachable!();
        }
    }
}

impl<'tcx, 'ctx, 'ctx1> BodyTransCtx<'tcx, 'ctx, 'ctx1> {
    /// Translate the type of a [mir::interpret::ConstValue::Scalar] value :
    /// Either a bool, a char, an integer, an enumeration ADT, an empty tuple or a static reference.
    fn translate_constant_scalar_type(&mut self, ty: &TyKind) -> ty::ETy {
        match ty {
            TyKind::Bool => ty::Ty::Literal(ty::LiteralTy::Bool),
            TyKind::Char => ty::Ty::Literal(ty::LiteralTy::Char),
            TyKind::Int(int_ty) => match int_ty {
                mir_ty::IntTy::Isize => {
                    ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::Isize))
                }
                mir_ty::IntTy::I8 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::I8)),
                mir_ty::IntTy::I16 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::I16)),
                mir_ty::IntTy::I32 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::I32)),
                mir_ty::IntTy::I64 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::I64)),
                mir_ty::IntTy::I128 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::I128)),
            },
            TyKind::Uint(uint_ty) => match uint_ty {
                mir_ty::UintTy::Usize => {
                    ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::Usize))
                }
                mir_ty::UintTy::U8 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::U8)),
                mir_ty::UintTy::U16 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::U16)),
                mir_ty::UintTy::U32 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::U32)),
                mir_ty::UintTy::U64 => ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::U64)),
                mir_ty::UintTy::U128 => {
                    ty::Ty::Literal(ty::LiteralTy::Integer(ty::IntegerTy::U128))
                }
            },
            TyKind::Adt(adt_def, substs) => {
                assert!(substs.is_empty());
                // It seems we can have ADTs when there is only one
                // variant, and this variant doesn't take parameters.
                // Retrieve the definition.
                let id = self.translate_type_id(adt_def.did());
                ty::Ty::Adt(id, Vec::new(), Vec::new(), Vec::new())
            }
            TyKind::Tuple(substs) => {
                // There can be tuple([]) for unit
                assert!(substs.is_empty());
                ty::Ty::Adt(ty::TypeId::Tuple, Vec::new(), Vec::new(), Vec::new())
            }
            // Only accept scalars that are shared references with erased regions : it's a static.
            TyKind::Ref(region, ref_ty, mir::Mutability::Not) => match region.kind() {
                mir_ty::RegionKind::ReErased => ty::Ty::Ref(
                    ty::ErasedRegion::Erased,
                    Box::new(self.translate_constant_scalar_type(ref_ty.kind())),
                    ty::RefKind::Shared,
                ),
                _ => unreachable!(),
            },
            TyKind::Float(_) => {
                // We don't support floating point numbers:
                // this should have been detected and eliminated before.
                unreachable!();
            }
            _ => {
                // The remaining types should not be used for constants, or
                // should have been filtered by the caller.
                error!("unexpected type: {:?}", ty);
                unreachable!();
            }
        }
    }

    /// Translate a parameter substitution.
    ///
    /// The regions parameters are expected to have been erased.
    fn translate_subst_with_erased_regions(
        &mut self,
        substs: &rustc_middle::ty::List<rustc_middle::ty::Ty<'tcx>>,
    ) -> Result<Vec<ty::ETy>> {
        let mut t_args_tys = Vec::new();

        for param in substs.iter() {
            t_args_tys.push(self.translate_ety(&param)?);
        }
        Ok(t_args_tys)
    }

    /// Translate the type of a [mir::interpret::ConstValue::ByRef] value.
    /// Currently, it should be a tuple.
    fn translate_constant_reference_type(&mut self, ty: &TyKind<'tcx>) -> ty::ETy {
        // Match on the type to destructure
        match ty {
            TyKind::Tuple(substs) => {
                // Here, the substitution only contains types (no regions)
                let type_params = self.translate_subst_with_erased_regions(substs).unwrap();
                trace!("{:?}", type_params);
                let field_tys = type_params.into_iter().collect();
                ty::Ty::Adt(ty::TypeId::Tuple, Vec::new(), field_tys, Vec::new())
            }
            TyKind::Adt(_, _) => {
                // Following tests, it seems rustc doesn't introduce constants
                // references when initializing ADTs, only when initializing tuples.
                // Anyway, our `OperandConstantValue` handles all cases so updating
                // the code to handle ADTs in a general manner wouldn't be a
                // problem.
                unreachable!("unexpected ADT type: {:?}", ty);
            }
            _ => {
                // The remaining types should not be used for constants, or
                // should have been filtered by the caller.
                unreachable!("unexpected type: {:?}", ty);
            }
        }
    }

    /// Translate a constant typed by [translate_constant_scalar_type].
    fn translate_constant_scalar_value(
        &mut self,
        llbc_ty: &ty::ETy,
        scalar: &mir::interpret::Scalar,
    ) -> e::OperandConstantValue {
        trace!("{:?}", scalar);

        // The documentation explicitly says not to match on a scalar.
        // A constant operand scalar is usually an instance of a primitive type
        // (bool, char, integer...). However, it may also be an instance of a
        // degenerate ADT or tuple (if an ADT has only one variant and no fields,
        // it is a constant, and unit is encoded by MIR as a 0-tuple).
        match llbc_ty {
            ty::Ty::Literal(ty::LiteralTy::Bool)
            | ty::Ty::Literal(ty::LiteralTy::Char)
            | ty::Ty::Literal(ty::LiteralTy::Integer(_)) => {
                let v = translate_constant_integer_like_value(llbc_ty, scalar);
                e::OperandConstantValue::Literal(v)
            }
            ty::Ty::Adt(ty::TypeId::Adt(id), region_tys, field_tys, cgs) => {
                assert!(region_tys.is_empty());
                assert!(field_tys.is_empty());
                assert!(cgs.is_empty());

                let def = self.t_ctx.type_defs.get(*id).unwrap();

                // Check that there is only one variant, with no fields
                // and no parameters. Construct the value at the same time.
                assert!(def.type_params.is_empty());
                let variant_id = match &def.kind {
                    ty::TypeDeclKind::Enum(variants) => {
                        assert!(variants.len() == 1);
                        Option::Some(ty::VariantId::ZERO)
                    }
                    ty::TypeDeclKind::Struct(_) => Option::None,
                    ty::TypeDeclKind::Opaque => {
                        unreachable!("Can't analyze a constant value built from an opaque type")
                    }
                };
                e::OperandConstantValue::Adt(variant_id, Vec::new())
            }
            ty::Ty::Adt(ty::TypeId::Tuple, region_tys, field_tys, cgs) => {
                assert!(region_tys.is_empty());
                assert!(field_tys.is_empty());
                assert!(cgs.is_empty());
                e::OperandConstantValue::Adt(Option::None, Vec::new())
            }
            ty::Ty::Ref(ty::ErasedRegion::Erased, _, ty::RefKind::Shared) => match scalar {
                mir::interpret::Scalar::Ptr(p, _) => {
                    match self.t_ctx.tcx.global_alloc(p.provenance) {
                        mir::interpret::GlobalAlloc::Static(s) => {
                            let id = self.translate_global_decl_id(s);
                            e::OperandConstantValue::StaticId(id)
                        }
                        _ => unreachable!(
                            "Expected static pointer, got {:?}",
                            self.t_ctx.tcx.global_alloc(p.provenance)
                        ),
                    }
                }
                _ => unreachable!("Expected static pointer, got {:?}", scalar),
            },
            _ => {
                // The remaining types should not be used for constants
                unreachable!("unexpected type: {:?}, for scalar: {:?}", llbc_ty, scalar);
            }
        }
    }

    /// Translate a constant typed by [translate_constant_reference_type].
    /// This should always be a tuple.
    fn translate_constant_reference_value(
        &mut self,
        llbc_ty: &ty::ETy,
        mir_ty: &Ty<'tcx>, // TODO: remove?
        value: &mir::interpret::ConstValue<'tcx>,
    ) -> e::OperandConstantValue {
        trace!();

        let tcx = self.t_ctx.tcx;

        // We use [try_destructure_mir_constant] to destructure the constant
        // We need a param_env: we use the function def id as a dummy id...
        let param_env = tcx.param_env(self.def_id);
        // We have to clone some values: it is a bit annoying, but I don't
        // manage to get the lifetimes working otherwise...
        let cvalue = rustc_middle::mir::ConstantKind::Val(*value, *mir_ty);
        let param_env_and_const = rustc_middle::ty::ParamEnvAnd {
            param_env,
            value: cvalue,
        };

        let dc = tcx
            .try_destructure_mir_constant(param_env_and_const)
            .unwrap();
        trace!("{:?}", dc);

        // Iterate over the fields
        assert!(dc.variant.is_none());

        // Below: we are mutually recursive with [translate_constant_kind],
        // which takes a [ConstantKind] as input (see `cvalue` above), but it should be
        // ok because we call it on a strictly smaller value.
        let fields: Vec<(ty::ETy, e::OperandConstantValue)> = dc
            .fields
            .iter()
            .map(|f| self.translate_constant_kind(f))
            .collect();

        // Sanity check
        match llbc_ty {
            ty::Ty::Adt(ty::TypeId::Tuple, regions, fields_tys, cgs) => {
                assert!(regions.is_empty());
                assert!(zip(&fields, fields_tys).all(|(f, ty)| &f.0 == ty));
                assert!(cgs.is_empty());
            }
            _ => unreachable!("Expected a tuple, got {:?}", mir_ty),
        };

        let fields: Vec<e::OperandConstantValue> = fields.into_iter().map(|f| f.1).collect();
        e::OperandConstantValue::Adt(Option::None, fields)
    }

    /// Translate a [mir::interpret::ConstValue]
    fn translate_const_value(
        &mut self,
        llbc_ty: &ty::ETy,
        mir_ty: &Ty<'tcx>, // TODO: remove?
        val: &mir::interpret::ConstValue<'tcx>,
    ) -> e::OperandConstantValue {
        trace!("{:?}", val);
        match val {
            mir::interpret::ConstValue::Scalar(scalar) => {
                self.translate_constant_scalar_value(llbc_ty, scalar)
            }
            mir::interpret::ConstValue::ByRef { .. } => {
                self.translate_constant_reference_value(llbc_ty, mir_ty, val)
            }
            mir::interpret::ConstValue::Slice { .. } => unimplemented!(),
            mir::interpret::ConstValue::ZeroSized { .. } => {
                // Should be unit
                assert!(llbc_ty.is_unit());
                e::OperandConstantValue::Adt(None, Vec::new())
            }
        }
    }

    /// This function translates a constant id, under the condition that the
    /// constants are extracted at the top level.
    fn translate_constant_id_as_top_level(
        &mut self,
        rid: DefId,
        mir_ty: &mir_ty::Ty<'tcx>,
    ) -> (ty::ETy, e::OperandConstantValue) {
        // Sanity check
        assert!(extract_constants_at_top_level(self.t_ctx.mir_level));

        // Lookup the constant identifier and refer to it.
        let id = self.translate_global_decl_id(rid);
        let ty = self.translate_ety(mir_ty).unwrap();
        (ty, e::OperandConstantValue::ConstantId(id))
    }

    fn translate_const_kind_unevaluated(
        &mut self,
        mir_ty: &mir_ty::Ty<'tcx>,
        ucv: &rustc_middle::mir::UnevaluatedConst<'tcx>,
    ) -> (ty::ETy, e::OperandConstantValue) {
        // Two cases:
        // - if we extract the constants at top level, we lookup the constant
        //   identifier and refer to it
        // - otherwise, we evaluate the constant and insert it in place
        if extract_constants_at_top_level(self.t_ctx.mir_level) {
            self.translate_constant_id_as_top_level(ucv.def, mir_ty)
        } else {
            // Evaluate the constant.
            // We need a param_env: we use the function def id as a dummy id...
            let tcx = self.t_ctx.tcx;
            let param_env = tcx.param_env(self.def_id);
            let cv = tcx.const_eval_resolve(param_env, *ucv, None).unwrap();
            let llbc_ty = self.translate_ety(mir_ty).unwrap();
            let v = self.translate_const_value(&llbc_ty, mir_ty, &cv);
            (llbc_ty, v)
        }
    }

    pub(crate) fn translate_const_kind(
        &mut self,
        constant: rustc_middle::ty::Const<'tcx>,
    ) -> (ty::ETy, e::OperandConstantValue) {
        match constant.kind() {
            ConstKind::Value(v) => {
                // The value is a [ValTree].
                // For now, we only imlement support for a limited subset of the cases -
                // there are many cases for which I don't know in which situations they
                // happen.

                // We only support integers and scalars
                let ty = self.translate_ety(&constant.ty()).unwrap();
                let v = match v {
                    mir_ty::ValTree::Leaf(v) => match ty.as_literal() {
                        ty::LiteralTy::Integer(int_ty) => {
                            if int_ty.is_signed() {
                                let v = v.try_to_int(v.size()).unwrap();
                                v::Literal::Scalar(v::ScalarValue::from_int(*int_ty, v).unwrap())
                            } else {
                                let v = v.try_to_uint(v.size()).unwrap();
                                v::Literal::Scalar(v::ScalarValue::from_uint(*int_ty, v).unwrap())
                            }
                        }
                        ty::LiteralTy::Bool => {
                            let v = v.try_to_bool().unwrap();
                            v::Literal::Bool(v)
                        }
                        ty::LiteralTy::Char => unimplemented!(),
                    },
                    mir_ty::ValTree::Branch(_) => {
                        // In practice I don't know when this is used
                        unimplemented!()
                    }
                };
                (ty, e::OperandConstantValue::Literal(v))
            }
            ConstKind::Expr(_) => {
                unimplemented!();
            }
            ConstKind::Unevaluated(ucv) => {
                // Two cases:
                // - if we extract the constants at top level, we lookup the constant
                //   identifier and refer to it
                // - otherwise, we evaluate the constant and insert it in place
                if extract_constants_at_top_level(self.t_ctx.mir_level) {
                    self.translate_constant_id_as_top_level(ucv.def, &constant.ty())
                } else {
                    // TODO: we can't call [translate_const_kind_unevaluated]:
                    // the types don't match.
                    // We could use [TyCtxt.const_eval_resolve_for_typeck]
                    // to get a [ValTree]
                    unimplemented!();
                }
            }
            ConstKind::Param(cp) => {
                let ty = self.translate_ety(&constant.ty()).unwrap();
                let cg_id = self.const_generic_vars_map.get(&cp.index).unwrap();
                (ty, e::OperandConstantValue::Var(*cg_id))
            }
            ConstKind::Infer(_)
            | ConstKind::Bound(_, _)
            | ConstKind::Placeholder(_)
            | ConstKind::Error(_) => {
                unreachable!("Unexpected: {:?}", constant);
            }
        }
    }

    pub(crate) fn translate_const_kind_as_const_generic(
        &mut self,
        constant: rustc_middle::ty::Const<'tcx>,
    ) -> ty::ConstGeneric {
        let (ty, c) = self.translate_const_kind(constant);
        assert!(ty.is_literal());
        match c {
            e::OperandConstantValue::Literal(v) => ty::ConstGeneric::Value(v),
            e::OperandConstantValue::Adt(..) => unreachable!(),
            e::OperandConstantValue::ConstantId(v) => ty::ConstGeneric::Global(v),
            e::OperandConstantValue::StaticId(_) => unreachable!(),
            e::OperandConstantValue::Var(v) => ty::ConstGeneric::Var(v),
        }
    }

    /// Translate a constant which may not be yet evaluated.
    pub(crate) fn translate_constant_kind(
        &mut self,
        constant: &rustc_middle::mir::ConstantKind<'tcx>,
    ) -> (ty::ETy, e::OperandConstantValue) {
        trace!("{:?}", constant);

        match constant {
            // This is the "normal" constant case
            // TODO: this changed when we updated from Nightly 2022-01-29 to
            // Nightly 2022-09-19, and the `Val` case used to be ignored.
            // SH: I'm not sure which corresponds to what (the documentation
            // is not super clear).
            mir::ConstantKind::Ty(c) => self.translate_const_kind(*c),
            // I'm not sure what this is about: the documentation is weird.
            mir::ConstantKind::Val(cv, ty) => {
                trace!("cv: {:?}, ty: {:?}", cv, ty);
                self.translate_evaluated_operand_constant(ty, cv)
            }
            rustc_middle::mir::ConstantKind::Unevaluated(ucv, mir_ty) => {
                self.translate_const_kind_unevaluated(mir_ty, ucv)
            }
        }
    }

    pub(crate) fn translate_evaluated_operand_constant(
        &mut self,
        ty: &Ty<'tcx>,
        val: &mir::interpret::ConstValue<'tcx>,
    ) -> (ty::ETy, e::OperandConstantValue) {
        let llbc_ty = self.translate_ety(ty).unwrap();
        let im_val = self.translate_const_value(&llbc_ty, ty, val);
        (llbc_ty, im_val)
    }

    /// Translate a constant which may not be yet evaluated.
    pub(crate) fn translate_operand_constant(
        &mut self,
        constant: &mir::Constant<'tcx>,
    ) -> (ty::ETy, e::OperandConstantValue) {
        trace!("{:?}", constant);
        use std::ops::Deref;
        let constant = &constant.deref();

        self.translate_constant_kind(&constant.literal)
    }
}
