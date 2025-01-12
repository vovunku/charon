//! This file groups everything which is linked to implementations about [crate::expressions]
#![allow(dead_code)]

use crate::assumed;
use crate::expressions::*;
use crate::formatter::Formatter;
use crate::gast::{AssumedFunId, Call, FunDeclId, FunId};
use crate::types::*;
use crate::ullbc_ast::GlobalDeclId;
use crate::values;
use crate::values::*;
use macros::make_generic_in_borrows;
use serde::{Serialize, Serializer};
use std::vec::Vec;

impl Place {
    pub fn new(var_id: VarId::Id) -> Place {
        Place {
            var_id,
            projection: Vec::new(),
        }
    }
}

impl std::fmt::Display for BorrowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            BorrowKind::Shared => write!(f, "Shared"),
            BorrowKind::Mut => write!(f, "Mut"),
            BorrowKind::TwoPhaseMut => write!(f, "TwoPhaseMut"),
            BorrowKind::Shallow => write!(f, "Shallow"),
        }
    }
}

impl std::fmt::Display for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            UnOp::Not => write!(f, "~"),
            UnOp::Neg => write!(f, "-"),
            UnOp::Cast(src, tgt) => write!(f, "cast<{src},{tgt}>"),
            UnOp::ArrayToSlice(..) => write!(f, "array_to_slice"),
        }
    }
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            BinOp::BitXor => write!(f, "^"),
            BinOp::BitAnd => write!(f, "&"),
            BinOp::BitOr => write!(f, "|"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Le => write!(f, "<="),
            BinOp::Ne => write!(f, "!="),
            BinOp::Ge => write!(f, ">="),
            BinOp::Gt => write!(f, ">"),
            BinOp::Div => write!(f, "/"),
            BinOp::Rem => write!(f, "%"),
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Shl => write!(f, "<<"),
            BinOp::Shr => write!(f, ">>"),
        }
    }
}

impl Place {
    pub fn fmt_with_ctx<T>(&self, ctx: &T) -> String
    where
        T: Formatter<TypeDeclId::Id>
            + Formatter<GlobalDeclId::Id>
            + Formatter<VarId::Id>
            + Formatter<(TypeDeclId::Id, Option<VariantId::Id>, FieldId::Id)>,
    {
        let mut out = ctx.format_object(self.var_id);

        for p in &self.projection {
            match p {
                ProjectionElem::Deref => {
                    out = format!("*({out})");
                }
                ProjectionElem::DerefBox => {
                    out = format!("deref_box ({out})");
                }
                ProjectionElem::DerefRawPtr => {
                    out = format!("deref_raw_ptr ({out})");
                }
                ProjectionElem::DerefPtrUnique => {
                    out = format!("deref_ptr_unique ({out})");
                }
                ProjectionElem::DerefPtrNonNull => {
                    out = format!("deref_ptr_non_null ({out})");
                }
                ProjectionElem::Field(proj_kind, field_id) => match proj_kind {
                    FieldProjKind::Adt(adt_id, opt_variant_id) => {
                        let field_name = ctx.format_object((*adt_id, *opt_variant_id, *field_id));
                        let downcast = match opt_variant_id {
                            None => "".to_string(),
                            Some(variant_id) => format!(" as variant @{variant_id}"),
                        };
                        out = format!("({out}{downcast}).{field_name}");
                    }
                    FieldProjKind::Tuple(_) => {
                        out = format!("({out}).{field_id}");
                    }
                    FieldProjKind::Option(_) => {
                        out = format!("({out}).{field_id}");
                    }
                },
                ProjectionElem::Index(i, _) => out = format!("({out})[{}]", ctx.format_object(*i)),
            }
        }

        out
    }

    /// Perform a type substitution - actually simply clone the object
    pub fn substitute(&self, _subst: &ETypeSubst) -> Self {
        self.clone()
    }
}

impl std::fmt::Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.fmt_with_ctx(&values::DummyFormatter {}))
    }
}

impl OperandConstantValue {
    pub fn fmt_with_ctx<T>(&self, ctx: &T) -> String
    where
        T: Formatter<TypeDeclId::Id>
            + Formatter<GlobalDeclId::Id>
            + Formatter<ConstGenericVarId::Id>,
    {
        match self {
            OperandConstantValue::Literal(c) => c.to_string(),
            OperandConstantValue::Adt(variant_id, values) => {
                // It is a bit annoying: in order to properly format the value,
                // we need the type (which contains the type def id).
                // Anyway, the printing utilities are mostly for debugging.
                let variant_id = match variant_id {
                    Option::Some(id) => format!("Some({id})"),
                    Option::None => "None".to_string(),
                };
                let values: Vec<String> = values.iter().map(|v| v.fmt_with_ctx(ctx)).collect();
                format!("ConstAdt {} [{}]", variant_id, values.join(", "))
            }
            OperandConstantValue::ConstantId(id) => ctx.format_object(*id),
            OperandConstantValue::StaticId(id) => format!("alloc: &{}", ctx.format_object(*id)),
            OperandConstantValue::Var(id) => format!("const {}", ctx.format_object(*id)),
        }
    }
}

impl std::fmt::Display for OperandConstantValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.fmt_with_ctx(&values::DummyFormatter {}))
    }
}

impl Operand {
    pub fn fmt_with_ctx<T>(&self, ctx: &T) -> String
    where
        T: Formatter<VarId::Id>
            + Formatter<TypeDeclId::Id>
            + Formatter<GlobalDeclId::Id>
            + Formatter<ConstGenericVarId::Id>
            + Formatter<(TypeDeclId::Id, Option<VariantId::Id>, FieldId::Id)>,
    {
        match self {
            Operand::Copy(p) => format!("copy ({})", p.fmt_with_ctx(ctx)),
            Operand::Move(p) => format!("move ({})", p.fmt_with_ctx(ctx)),
            Operand::Const(_, c) => format!("const ({})", c.fmt_with_ctx(ctx)),
        }
    }

    /// Perform a type substitution - actually simply clone the object
    pub fn substitute(&self, _subst: &ETypeSubst) -> Self {
        self.clone()
    }
}

impl std::fmt::Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.fmt_with_ctx(&values::DummyFormatter {}))
    }
}

impl Rvalue {
    pub fn fmt_with_ctx<'a, T>(&'a self, ctx: &T) -> String
    where
        T: Formatter<VarId::Id>
            + Formatter<TypeDeclId::Id>
            + Formatter<GlobalDeclId::Id>
            + Formatter<(TypeDeclId::Id, VariantId::Id)>
            + Formatter<(TypeDeclId::Id, Option<VariantId::Id>, FieldId::Id)>
            + Formatter<TypeVarId::Id>
            + Formatter<ConstGenericVarId::Id>
            + Formatter<&'a ErasedRegion>,
    {
        match self {
            Rvalue::Use(x) => x.fmt_with_ctx(ctx),
            Rvalue::Ref(place, borrow_kind) => match borrow_kind {
                BorrowKind::Shared => format!("&{}", place.fmt_with_ctx(ctx)),
                BorrowKind::Mut => format!("&mut {}", place.fmt_with_ctx(ctx)),
                BorrowKind::TwoPhaseMut => {
                    format!("&two-phase-mut {}", place.fmt_with_ctx(ctx))
                }
                BorrowKind::Shallow => format!("&shallow {}", place.fmt_with_ctx(ctx)),
            },
            Rvalue::UnaryOp(unop, x) => {
                format!("{}({})", unop, x.fmt_with_ctx(ctx))
            }
            Rvalue::BinaryOp(binop, x, y) => {
                format!("{} {} {}", x.fmt_with_ctx(ctx), binop, y.fmt_with_ctx(ctx))
            }
            Rvalue::Discriminant(p) => {
                format!("@discriminant({})", p.fmt_with_ctx(ctx),)
            }
            Rvalue::Aggregate(kind, ops) => {
                let ops_s: Vec<String> = ops.iter().map(|op| op.fmt_with_ctx(ctx)).collect();
                match kind {
                    AggregateKind::Tuple => format!("({})", ops_s.join(", ")),
                    AggregateKind::Option(variant_id, _) => {
                        if *variant_id == assumed::OPTION_NONE_VARIANT_ID {
                            assert!(ops.is_empty());
                            "@Option::None".to_string()
                        } else if *variant_id == assumed::OPTION_SOME_VARIANT_ID {
                            assert!(ops.len() == 1);
                            format!("@Option::Some({})", ops[0].fmt_with_ctx(ctx))
                        } else {
                            unreachable!();
                        }
                    }
                    AggregateKind::Adt(def_id, variant_id, _, _, _) => {
                        // Format every field
                        let mut fields = vec![];
                        for (i, op) in ops.iter().enumerate() {
                            let field_id = FieldId::Id::new(i);
                            let field_name = ctx.format_object((*def_id, *variant_id, field_id));
                            fields.push(format!("{}: {}", field_name, op.fmt_with_ctx(ctx)));
                        }

                        let variant = match variant_id {
                            None => ctx.format_object(*def_id),
                            Some(variant_id) => ctx.format_object((*def_id, *variant_id)),
                        };
                        format!("{} {{ {} }}", variant, fields.join(", "))
                    }
                    AggregateKind::Array(_, _) => {
                        format!("[{}]", ops_s.join(", "))
                    }
                    AggregateKind::Range(_) => {
                        format!("@Range[{}]", ops_s.join(", "))
                    }
                }
            }
            Rvalue::Global(gid) => ctx.format_object(*gid),
            Rvalue::Len(place, ..) => format!("len({})", place.fmt_with_ctx(ctx)),
        }
    }

    /// Perform a type substitution - actually simply clone the object
    pub fn substitute(&self, _subst: &ETypeSubst) -> Self {
        self.clone()
    }
}

impl std::fmt::Display for Rvalue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.fmt_with_ctx(&values::DummyFormatter {}))
    }
}

impl Serialize for OperandConstantValue {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            // [OperandConstantValue] exists only to handle temporary cases inherited from the MIR:
            // for the final (U)LLBC format, we simply export the underlying constant value.
            OperandConstantValue::Literal(cv) => cv.serialize(serializer),
            _ => unreachable!("unexpected `{:?}`: `OperandConstantValue` fields other than `ConstantValue` are temporary and should not occur in serialized LLBC", self),
        }
    }
}

// Derive two implementations at once: one which uses shared borrows, and one
// which uses mutable borrows.
// Generates the traits: `SharedExprVisitor` and `MutExprVisitor`.
make_generic_in_borrows! {

/// A visitor for expressions.
///
/// TODO: implement macros to automatically derive visitors.
pub trait ExprVisitor: crate::types::TypeVisitor {
    fn visit_place(&mut self, p: &Place) {
        self.visit_var_id(&p.var_id);
        self.visit_projection(&p.projection);
    }

    fn visit_var_id(&mut self, _: &VarId::Id) {}

    fn visit_projection(&mut self, p: &Projection) {
        for pe in p.iter() {
            self.visit_projection_elem(pe)
        }
    }

    fn default_visit_projection_elem(&mut self, pe: &ProjectionElem) {
        match pe {
            ProjectionElem::Deref => self.visit_deref(),
            ProjectionElem::DerefBox => self.visit_deref_box(),
            ProjectionElem::DerefRawPtr => self.visit_deref_raw_ptr(),
            ProjectionElem::DerefPtrUnique => self.visit_deref_ptr_unique(),
            ProjectionElem::DerefPtrNonNull => self.visit_deref_ptr_non_null(),
            ProjectionElem::Field(proj_kind, fid) => self.visit_projection_field(proj_kind, fid),
            ProjectionElem::Index(i, _) => self.visit_var_id(i),
        }
    }

    fn visit_projection_elem(&mut self, pe: &ProjectionElem) {
        self.default_visit_projection_elem(pe)
    }

    fn visit_deref(&mut self) {}
    fn visit_deref_box(&mut self) {}
    fn visit_deref_raw_ptr(&mut self) {}
    fn visit_deref_ptr_unique(&mut self) {}
    fn visit_deref_ptr_non_null(&mut self) {}
    fn visit_projection_field(&mut self, _: &FieldProjKind, _: &FieldId::Id) {}

    fn default_visit_operand(&mut self, o: &Operand) {
        match o {
            Operand::Copy(p) => self.visit_copy(p),
            Operand::Move(p) => self.visit_move(p),
            Operand::Const(ety, cv) => self.visit_operand_const(ety, cv),
        }
    }

    fn visit_operand(&mut self, o: &Operand) {
        self.default_visit_operand(o)
    }

    fn visit_copy(&mut self, p: &Place) {
        self.visit_place(p)
    }

    fn visit_move(&mut self, p: &Place) {
        self.visit_place(p)
    }

    fn visit_operand_const(&mut self, ty: &ETy, op: &OperandConstantValue) {
        self.visit_ty(ty);
        self.visit_operand_constant_value(op);
    }

    fn visit_operand_constant_value(&mut self, op: &OperandConstantValue) {
        use OperandConstantValue::*;
        match op {
            Literal(lit) => self.visit_literal(lit),
            Adt(oid, ops) => self.visit_operand_const_adt(oid, ops),
            ConstantId(id) => self.visit_global_decl_id(id),
            StaticId(id) => self.visit_global_decl_id(id),
            Var(id) => self.visit_const_generic_var_id(id),
        }
    }

    fn visit_operand_const_adt(
        &mut self,
        _oid: &Option<VariantId::Id>,
        ops: &Vec<OperandConstantValue>,
    ) {
        for op in ops {
            self.visit_operand_constant_value(op)
        }
    }

    fn default_visit_rvalue(&mut self, rv: &Rvalue) {
        match rv {
            Rvalue::Use(o) => self.visit_use(o),
            Rvalue::Ref(p, bkind) => self.visit_ref(p, bkind),
            Rvalue::UnaryOp(op, o1) => self.visit_unary_op(op, o1),
            Rvalue::BinaryOp(op, o1, o2) => self.visit_binary_op(op, o1, o2),
            Rvalue::Discriminant(p) => self.visit_discriminant(p),
            Rvalue::Aggregate(kind, ops) => self.visit_aggregate(kind, ops),
            Rvalue::Global(gid) => self.visit_global(gid),
            Rvalue::Len(p, ty, cg) => self.visit_len(p, ty, cg),
        }
    }

    fn visit_rvalue(&mut self, o: &Rvalue) {
        self.default_visit_rvalue(o)
    }

    fn visit_use(&mut self, o: &Operand) {
        self.visit_operand(o)
    }

    fn visit_ref(&mut self, p: &Place, _: &BorrowKind) {
        self.visit_place(p)
    }

    fn visit_unary_op(&mut self, _: &UnOp, o1: &Operand) {
        self.visit_operand(o1)
    }

    fn visit_binary_op(&mut self, _: &BinOp, o1: &Operand, o2: &Operand) {
        self.visit_operand(o1);
        self.visit_operand(o2);
    }

    fn visit_discriminant(&mut self, p: &Place) {
        self.visit_place(p)
    }

    fn visit_aggregate(&mut self, ak: &AggregateKind, ops: &Vec<Operand>) {
        self.visit_aggregate_kind(ak);
        for o in ops {
            self.visit_operand(o)
        }
    }

    fn visit_aggregate_kind(&mut self, ak: &AggregateKind) {
        use AggregateKind::*;
        // We could generalize and introduce auxiliary functions for
        // the various cases - this is not necessary for now
        match ak {
            Tuple => (),
            Option(_, ty) => self.visit_ty(ty),
            Range(ty) => self.visit_ty(ty),
            Adt(adt_id, _, _, tys, cgs) => {
                self.visit_type_decl_id(adt_id);
                for ty in tys {
                    self.visit_ty(ty);
                }
                for cg in cgs {
                    self.visit_const_generic(cg);
                }
            }
            Array(ty, cg) => {
                self.visit_ty(ty);
                self.visit_const_generic(cg);
            }
        }
    }

    fn visit_global(&mut self, _: &GlobalDeclId::Id) {}

    fn visit_len(&mut self, p: &Place, _ty: &ETy, _cg: &Option<ConstGeneric>) {
        self.visit_place(p)
    }

    fn visit_call(&mut self, c: &Call) {
        self.visit_fun_id(&c.func);
        // We ignore the regions which are erased
        for t in &c.type_args {
            self.visit_ty(t);
        }
        for cg in &c.const_generic_args {
            self.visit_const_generic(cg);
        }
        for o in &c.args {
            self.visit_operand(o);
        }
        self.visit_place(&c.dest);
    }

    fn visit_fun_id(&mut self, fun_id: &FunId) {
        match fun_id {
            FunId::Regular(fid) => self.visit_fun_decl_id(fid),
            FunId::Assumed(aid) => self.visit_assumed_fun_id(aid),
        }
    }

    fn visit_fun_decl_id(&mut self, fid: &FunDeclId::Id) {}
    fn visit_assumed_fun_id(&mut self, fid: &AssumedFunId) {}
}

} // make_generic_in_borrows
