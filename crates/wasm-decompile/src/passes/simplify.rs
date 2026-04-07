//! Simplification pass
//!
//! Performs constant folding, algebraic simplification, and dead code elimination.

use crate::ir::*;
use crate::passes::{Pass, PassContext};

/// Simplification pass
pub struct SimplifyPass;

impl Pass for SimplifyPass {
    fn name(&self) -> &'static str {
        "simplify"
    }

    fn run(&self, module: &mut Module, _ctx: &mut PassContext) {
        for func in &mut module.functions {
            if !func.is_import {
                simplify_block(&mut func.body);
            }
        }
    }
}

fn simplify_block(block: &mut Block) {
    // First pass: simplify each statement
    for stmt in &mut block.stmts {
        simplify_stmt(stmt);
    }

    // Second pass: remove dead code
    block.stmts.retain(|stmt| !is_dead_stmt(stmt));

    // Third pass: convert block+br_if patterns to if statements
    simplify_block_to_if(block);
}

/// Convert block+br_if patterns to if statements
///
/// Pattern:
/// ```ignore
/// block_X: {
///     if (COND) break block_X;
///     ...rest...
/// }
/// ```
///
/// Transforms to:
/// ```ignore
/// if (!COND) {
///     ...rest...
/// }
/// ```
fn simplify_block_to_if(block: &mut Block) {
    let mut new_stmts = Vec::with_capacity(block.stmts.len());

    for stmt in std::mem::take(&mut block.stmts) {
        if let Stmt::Block { label, body } = stmt {
            // Check if this block can be transformed
            if can_transform_block_to_if(&body, label) {
                // Transform: extract condition and rest of body
                let mut body_stmts = body.stmts;
                if let Some(Stmt::BrIf { cond, .. }) = body_stmts.first().cloned() {
                    // Remove the BrIf
                    body_stmts.remove(0);

                    // Negate the condition and create an if statement
                    let negated_cond = negate_condition(cond);
                    let then_block = Block::with_stmts(body_stmts);

                    new_stmts.push(Stmt::If {
                        cond: negated_cond,
                        then_block,
                        else_block: None,
                    });
                    continue;
                }
                // Couldn't extract BrIf (shouldn't happen), keep as-is
                new_stmts.push(Stmt::Block {
                    label,
                    body: Block::with_stmts(body_stmts),
                });
            } else {
                // Couldn't transform, keep as-is
                new_stmts.push(Stmt::Block { label, body });
            }
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
}

/// Check if a block can be transformed to an if statement
fn can_transform_block_to_if(body: &Block, label: u32) -> bool {
    // Must have at least 2 statements (BrIf + something)
    if body.stmts.len() < 2 {
        return false;
    }

    // First statement must be BrIf to the same label (not a loop)
    match body.stmts.first() {
        Some(Stmt::BrIf {
            label: br_label,
            is_loop: false,
            ..
        }) if *br_label == label => {}
        _ => return false,
    }

    // Rest of body must not reference this label
    let rest = &body.stmts[1..];
    !has_branch_to_label(rest, label)
}

/// Check if any statement references the given label
fn has_branch_to_label(stmts: &[Stmt], label: u32) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Br { label: l, .. } | Stmt::BrIf { label: l, .. } if *l == label => {
                return true;
            }
            Stmt::BrTable {
                targets, default, ..
            } => {
                if targets.iter().any(|t| t.label == label) || default.label == label {
                    return true;
                }
            }
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                if has_branch_to_label(&body.stmts, label) {
                    return true;
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if has_branch_to_label(&then_block.stmts, label) {
                    return true;
                }
                if let Some(eb) = else_block {
                    if has_branch_to_label(&eb.stmts, label) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Negate a condition expression
///
/// For comparisons, inverts the operator directly.
/// For Eqz (logical NOT), removes it (double negation).
/// For other expressions, wraps in Eqz.
fn negate_condition(cond: Expr) -> Expr {
    match cond.kind {
        // Invert comparison operators
        ExprKind::Compare(op, a, b, operand_ty) => {
            let negated_op = match op {
                CmpOp::Eq => CmpOp::Ne,
                CmpOp::Ne => CmpOp::Eq,
                CmpOp::LtS => CmpOp::GeS,
                CmpOp::LtU => CmpOp::GeU,
                CmpOp::GtS => CmpOp::LeS,
                CmpOp::GtU => CmpOp::LeU,
                CmpOp::LeS => CmpOp::GtS,
                CmpOp::LeU => CmpOp::GtU,
                CmpOp::GeS => CmpOp::LtS,
                CmpOp::GeU => CmpOp::LtU,
                // Float comparisons
                CmpOp::FEq => CmpOp::FNe,
                CmpOp::FNe => CmpOp::FEq,
                CmpOp::FLt => CmpOp::FGe,
                CmpOp::FGt => CmpOp::FLe,
                CmpOp::FLe => CmpOp::FGt,
                CmpOp::FGe => CmpOp::FLt,
            };
            Expr::with_type(
                ExprKind::Compare(negated_op, a, b, operand_ty),
                InferredType::Bool,
            )
        }

        // Double negation: Eqz(x) negated -> x
        ExprKind::UnaryOp(UnaryOp::Eqz, inner) => *inner,

        // Default: wrap in Eqz (which is == 0, i.e., logical NOT)
        _ => Expr::with_type(
            ExprKind::UnaryOp(UnaryOp::Eqz, Box::new(cond)),
            InferredType::Bool,
        ),
    }
}

fn simplify_stmt(stmt: &mut Stmt) {
    match stmt {
        Stmt::LocalSet { value, .. } => {
            *value = simplify_expr(std::mem::replace(value, Expr::i32_const(0)));
        }
        Stmt::GlobalSet { value, .. } => {
            *value = simplify_expr(std::mem::replace(value, Expr::i32_const(0)));
        }
        Stmt::Store { addr, value, .. } => {
            *addr = simplify_expr(std::mem::replace(addr, Expr::i32_const(0)));
            *value = simplify_expr(std::mem::replace(value, Expr::i32_const(0)));
        }
        Stmt::Expr(expr) => {
            *expr = simplify_expr(std::mem::replace(expr, Expr::i32_const(0)));
        }
        Stmt::Return(Some(expr)) => {
            *expr = simplify_expr(std::mem::replace(expr, Expr::i32_const(0)));
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            *cond = simplify_expr(std::mem::replace(cond, Expr::i32_const(0)));
            simplify_block(then_block);
            if let Some(else_blk) = else_block {
                simplify_block(else_blk);
            }
        }
        Stmt::Block { body, .. } => {
            simplify_block(body);
        }
        Stmt::Loop { body, .. } => {
            simplify_block(body);
        }
        Stmt::DoWhile { body, cond } => {
            simplify_block(body);
            *cond = simplify_expr(std::mem::replace(cond, Expr::i32_const(0)));
        }
        Stmt::While { cond, body } => {
            *cond = simplify_expr(std::mem::replace(cond, Expr::i32_const(0)));
            simplify_block(body);
        }
        Stmt::BrIf { cond, .. } => {
            *cond = simplify_expr(std::mem::replace(cond, Expr::i32_const(0)));
        }
        Stmt::BrTable { index, .. } => {
            *index = simplify_expr(std::mem::replace(index, Expr::i32_const(0)));
        }
        Stmt::Drop(expr) => {
            *expr = simplify_expr(std::mem::replace(expr, Expr::i32_const(0)));
        }
        _ => {}
    }
}

fn simplify_expr(expr: Expr) -> Expr {
    match expr.kind {
        ExprKind::BinOp(op, a, b) => {
            let a = simplify_expr(*a);
            let b = simplify_expr(*b);

            // Constant folding
            if let Some(result) = fold_binop(op, &a, &b) {
                return result;
            }

            // Algebraic simplifications
            if let Some(result) = algebraic_simplify(op, &a, &b) {
                return result;
            }

            Expr::with_type(ExprKind::BinOp(op, Box::new(a), Box::new(b)), expr.ty)
        }

        ExprKind::UnaryOp(op, a) => {
            let a = simplify_expr(*a);

            // Constant folding for unary ops
            if let Some(result) = fold_unaryop(op, &a) {
                return result;
            }

            Expr::with_type(ExprKind::UnaryOp(op, Box::new(a)), expr.ty)
        }

        ExprKind::Compare(op, a, b, operand_ty) => {
            let a = simplify_expr(*a);
            let b = simplify_expr(*b);

            // Constant folding for comparisons
            if let Some(result) = fold_cmpop(op, &a, &b) {
                return result;
            }

            Expr::with_type(
                ExprKind::Compare(op, Box::new(a), Box::new(b), operand_ty),
                expr.ty,
            )
        }

        ExprKind::Load {
            addr,
            offset,
            size,
            signed,
        } => {
            let addr = simplify_expr(*addr);
            Expr::with_type(
                ExprKind::Load {
                    addr: Box::new(addr),
                    offset,
                    size,
                    signed,
                },
                expr.ty,
            )
        }

        ExprKind::Call { func, args } => {
            let args: Vec<Expr> = args.into_iter().map(simplify_expr).collect();
            Expr::with_type(ExprKind::Call { func, args }, expr.ty)
        }

        ExprKind::CallIndirect {
            type_idx,
            table_idx,
            index,
            args,
        } => {
            let index = simplify_expr(*index);
            let args: Vec<Expr> = args.into_iter().map(simplify_expr).collect();
            Expr::with_type(
                ExprKind::CallIndirect {
                    type_idx,
                    table_idx,
                    index: Box::new(index),
                    args,
                },
                expr.ty,
            )
        }

        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => {
            let cond = simplify_expr(*cond);
            let then_val = simplify_expr(*then_val);
            let else_val = simplify_expr(*else_val);

            // If condition is constant, pick the right branch
            if let ExprKind::I32Const(v) = cond.kind {
                return if v != 0 { then_val } else { else_val };
            }

            Expr::with_type(
                ExprKind::Select {
                    cond: Box::new(cond),
                    then_val: Box::new(then_val),
                    else_val: Box::new(else_val),
                },
                expr.ty,
            )
        }

        ExprKind::Convert { op, expr: inner } => {
            let inner = simplify_expr(*inner);

            // Fold constant conversions
            if let Some(result) = fold_convert(op, &inner) {
                return result;
            }

            Expr::with_type(
                ExprKind::Convert {
                    op,
                    expr: Box::new(inner),
                },
                expr.ty,
            )
        }

        ExprKind::GoString { ptr, len } => {
            let ptr = simplify_expr(*ptr);
            let len = simplify_expr(*len);
            Expr::with_type(
                ExprKind::GoString {
                    ptr: Box::new(ptr),
                    len: Box::new(len),
                },
                expr.ty,
            )
        }

        ExprKind::GoSlice { ptr, len, cap } => {
            let ptr = simplify_expr(*ptr);
            let len = simplify_expr(*len);
            let cap = simplify_expr(*cap);
            Expr::with_type(
                ExprKind::GoSlice {
                    ptr: Box::new(ptr),
                    len: Box::new(len),
                    cap: Box::new(cap),
                },
                expr.ty,
            )
        }

        ExprKind::GoInterface { type_ptr, data } => {
            let type_ptr = simplify_expr(*type_ptr);
            let data = simplify_expr(*data);
            Expr::with_type(
                ExprKind::GoInterface {
                    type_ptr: Box::new(type_ptr),
                    data: Box::new(data),
                },
                expr.ty,
            )
        }

        // Already simplified or atomic
        _ => expr,
    }
}

/// Constant folding for binary operations
fn fold_binop(op: BinOp, a: &Expr, b: &Expr) -> Option<Expr> {
    match (&a.kind, &b.kind) {
        (ExprKind::I32Const(av), ExprKind::I32Const(bv)) => {
            let result = match op {
                BinOp::Add => av.wrapping_add(*bv),
                BinOp::Sub => av.wrapping_sub(*bv),
                BinOp::Mul => av.wrapping_mul(*bv),
                BinOp::DivS if *bv != 0 => av.wrapping_div(*bv),
                BinOp::DivU if *bv != 0 => (*av as u32).wrapping_div(*bv as u32) as i32,
                BinOp::RemS if *bv != 0 => av.wrapping_rem(*bv),
                BinOp::RemU if *bv != 0 => (*av as u32).wrapping_rem(*bv as u32) as i32,
                BinOp::And => *av & *bv,
                BinOp::Or => *av | *bv,
                BinOp::Xor => *av ^ *bv,
                BinOp::Shl => av.wrapping_shl(*bv as u32),
                BinOp::ShrS => av.wrapping_shr(*bv as u32),
                BinOp::ShrU => ((*av as u32).wrapping_shr(*bv as u32)) as i32,
                BinOp::Rotl => av.rotate_left(*bv as u32),
                BinOp::Rotr => av.rotate_right(*bv as u32),
                _ => return None,
            };
            Some(Expr::i32_const(result))
        }

        (ExprKind::I64Const(av), ExprKind::I64Const(bv)) => {
            let result = match op {
                BinOp::Add => av.wrapping_add(*bv),
                BinOp::Sub => av.wrapping_sub(*bv),
                BinOp::Mul => av.wrapping_mul(*bv),
                BinOp::DivS if *bv != 0 => av.wrapping_div(*bv),
                BinOp::DivU if *bv != 0 => (*av as u64).wrapping_div(*bv as u64) as i64,
                BinOp::RemS if *bv != 0 => av.wrapping_rem(*bv),
                BinOp::RemU if *bv != 0 => (*av as u64).wrapping_rem(*bv as u64) as i64,
                BinOp::And => *av & *bv,
                BinOp::Or => *av | *bv,
                BinOp::Xor => *av ^ *bv,
                BinOp::Shl => av.wrapping_shl(*bv as u32),
                BinOp::ShrS => av.wrapping_shr(*bv as u32),
                BinOp::ShrU => ((*av as u64).wrapping_shr(*bv as u32)) as i64,
                BinOp::Rotl => av.rotate_left(*bv as u32),
                BinOp::Rotr => av.rotate_right(*bv as u32),
                _ => return None,
            };
            Some(Expr::i64_const(result))
        }

        (ExprKind::F32Const(av), ExprKind::F32Const(bv)) => {
            let result = match op {
                BinOp::FAdd => av + bv,
                BinOp::FSub => av - bv,
                BinOp::FMul => av * bv,
                BinOp::FDiv => av / bv,
                BinOp::FMin => av.min(*bv),
                BinOp::FMax => av.max(*bv),
                _ => return None,
            };
            Some(Expr::f32_const(result))
        }

        (ExprKind::F64Const(av), ExprKind::F64Const(bv)) => {
            let result = match op {
                BinOp::FAdd => av + bv,
                BinOp::FSub => av - bv,
                BinOp::FMul => av * bv,
                BinOp::FDiv => av / bv,
                BinOp::FMin => av.min(*bv),
                BinOp::FMax => av.max(*bv),
                _ => return None,
            };
            Some(Expr::f64_const(result))
        }

        _ => None,
    }
}

/// Algebraic simplifications
fn algebraic_simplify(op: BinOp, a: &Expr, b: &Expr) -> Option<Expr> {
    // x + 0 -> x
    // x - 0 -> x
    // x * 1 -> x
    // x | 0 -> x
    // x ^ 0 -> x
    // x & 0xFFFFFFFF -> x (for i32)

    match op {
        BinOp::Add | BinOp::Sub | BinOp::Or | BinOp::Xor => {
            if matches!(b.kind, ExprKind::I32Const(0) | ExprKind::I64Const(0)) {
                return Some(a.clone());
            }
            if matches!(a.kind, ExprKind::I32Const(0) | ExprKind::I64Const(0))
                && matches!(op, BinOp::Add | BinOp::Or | BinOp::Xor)
            {
                return Some(b.clone());
            }
        }

        BinOp::Mul => {
            // x * 0 -> 0
            if matches!(b.kind, ExprKind::I32Const(0)) {
                return Some(Expr::i32_const(0));
            }
            if matches!(a.kind, ExprKind::I32Const(0)) {
                return Some(Expr::i32_const(0));
            }
            if matches!(b.kind, ExprKind::I64Const(0)) {
                return Some(Expr::i64_const(0));
            }
            if matches!(a.kind, ExprKind::I64Const(0)) {
                return Some(Expr::i64_const(0));
            }
            // x * 1 -> x
            if matches!(b.kind, ExprKind::I32Const(1) | ExprKind::I64Const(1)) {
                return Some(a.clone());
            }
            if matches!(a.kind, ExprKind::I32Const(1) | ExprKind::I64Const(1)) {
                return Some(b.clone());
            }
        }

        BinOp::And => {
            // x & 0 -> 0
            if matches!(b.kind, ExprKind::I32Const(0)) {
                return Some(Expr::i32_const(0));
            }
            if matches!(b.kind, ExprKind::I64Const(0)) {
                return Some(Expr::i64_const(0));
            }
            // x & 0xFFFFFFFF -> x (for i32)
            if matches!(b.kind, ExprKind::I32Const(-1)) {
                return Some(a.clone());
            }
            if matches!(b.kind, ExprKind::I64Const(-1)) {
                return Some(a.clone());
            }
        }

        BinOp::Shl | BinOp::ShrS | BinOp::ShrU => {
            // x << 0 -> x
            // x >> 0 -> x
            if matches!(b.kind, ExprKind::I32Const(0) | ExprKind::I64Const(0)) {
                return Some(a.clone());
            }
        }

        _ => {}
    }

    None
}

/// Constant folding for unary operations
fn fold_unaryop(op: UnaryOp, a: &Expr) -> Option<Expr> {
    match &a.kind {
        ExprKind::I32Const(v) => {
            let result = match op {
                UnaryOp::Clz => v.leading_zeros() as i32,
                UnaryOp::Ctz => v.trailing_zeros() as i32,
                UnaryOp::Popcnt => v.count_ones() as i32,
                UnaryOp::Eqz => {
                    if *v == 0 {
                        1
                    } else {
                        0
                    }
                }
                _ => return None,
            };
            Some(Expr::i32_const(result))
        }

        ExprKind::I64Const(v) => {
            let result: i64 = match op {
                UnaryOp::Clz => v.leading_zeros() as i64,
                UnaryOp::Ctz => v.trailing_zeros() as i64,
                UnaryOp::Popcnt => v.count_ones() as i64,
                UnaryOp::Eqz => {
                    if *v == 0 {
                        1
                    } else {
                        0
                    }
                }
                _ => return None,
            };
            Some(Expr::i64_const(result))
        }

        ExprKind::F32Const(v) => {
            let result = match op {
                UnaryOp::FAbs => v.abs(),
                UnaryOp::FNeg => -v,
                UnaryOp::FCeil => v.ceil(),
                UnaryOp::FFloor => v.floor(),
                UnaryOp::FTrunc => v.trunc(),
                UnaryOp::FNearest => v.round(),
                UnaryOp::FSqrt => v.sqrt(),
                _ => return None,
            };
            Some(Expr::f32_const(result))
        }

        ExprKind::F64Const(v) => {
            let result = match op {
                UnaryOp::FAbs => v.abs(),
                UnaryOp::FNeg => -v,
                UnaryOp::FCeil => v.ceil(),
                UnaryOp::FFloor => v.floor(),
                UnaryOp::FTrunc => v.trunc(),
                UnaryOp::FNearest => v.round(),
                UnaryOp::FSqrt => v.sqrt(),
                _ => return None,
            };
            Some(Expr::f64_const(result))
        }

        _ => None,
    }
}

/// Constant folding for comparisons
fn fold_cmpop(op: CmpOp, a: &Expr, b: &Expr) -> Option<Expr> {
    match (&a.kind, &b.kind) {
        (ExprKind::I32Const(av), ExprKind::I32Const(bv)) => {
            let result = match op {
                CmpOp::Eq => av == bv,
                CmpOp::Ne => av != bv,
                CmpOp::LtS => av < bv,
                CmpOp::LtU => (*av as u32) < (*bv as u32),
                CmpOp::GtS => av > bv,
                CmpOp::GtU => (*av as u32) > (*bv as u32),
                CmpOp::LeS => av <= bv,
                CmpOp::LeU => (*av as u32) <= (*bv as u32),
                CmpOp::GeS => av >= bv,
                CmpOp::GeU => (*av as u32) >= (*bv as u32),
                _ => return None,
            };
            Some(Expr::i32_const(if result { 1 } else { 0 }))
        }

        (ExprKind::I64Const(av), ExprKind::I64Const(bv)) => {
            let result = match op {
                CmpOp::Eq => av == bv,
                CmpOp::Ne => av != bv,
                CmpOp::LtS => av < bv,
                CmpOp::LtU => (*av as u64) < (*bv as u64),
                CmpOp::GtS => av > bv,
                CmpOp::GtU => (*av as u64) > (*bv as u64),
                CmpOp::LeS => av <= bv,
                CmpOp::LeU => (*av as u64) <= (*bv as u64),
                CmpOp::GeS => av >= bv,
                CmpOp::GeU => (*av as u64) >= (*bv as u64),
                _ => return None,
            };
            Some(Expr::i32_const(if result { 1 } else { 0 }))
        }

        (ExprKind::F32Const(av), ExprKind::F32Const(bv)) => {
            let result = match op {
                CmpOp::FEq => av == bv,
                CmpOp::FNe => av != bv,
                CmpOp::FLt => av < bv,
                CmpOp::FGt => av > bv,
                CmpOp::FLe => av <= bv,
                CmpOp::FGe => av >= bv,
                _ => return None,
            };
            Some(Expr::i32_const(if result { 1 } else { 0 }))
        }

        (ExprKind::F64Const(av), ExprKind::F64Const(bv)) => {
            let result = match op {
                CmpOp::FEq => av == bv,
                CmpOp::FNe => av != bv,
                CmpOp::FLt => av < bv,
                CmpOp::FGt => av > bv,
                CmpOp::FLe => av <= bv,
                CmpOp::FGe => av >= bv,
                _ => return None,
            };
            Some(Expr::i32_const(if result { 1 } else { 0 }))
        }

        _ => None,
    }
}

/// Constant folding for conversions
fn fold_convert(op: ConvertOp, inner: &Expr) -> Option<Expr> {
    match &inner.kind {
        ExprKind::I32Const(v) => match op {
            ConvertOp::I64ExtendI32S => Some(Expr::i64_const(*v as i64)),
            ConvertOp::I64ExtendI32U => Some(Expr::i64_const(*v as u32 as i64)),
            ConvertOp::F32ConvertI32S => Some(Expr::f32_const(*v as f32)),
            ConvertOp::F32ConvertI32U => Some(Expr::f32_const(*v as u32 as f32)),
            ConvertOp::F64ConvertI32S => Some(Expr::f64_const(*v as f64)),
            ConvertOp::F64ConvertI32U => Some(Expr::f64_const(*v as u32 as f64)),
            ConvertOp::I32Extend8S => Some(Expr::i32_const(*v as i8 as i32)),
            ConvertOp::I32Extend16S => Some(Expr::i32_const(*v as i16 as i32)),
            ConvertOp::F32ReinterpretI32 => Some(Expr::f32_const(f32::from_bits(*v as u32))),
            _ => None,
        },

        ExprKind::I64Const(v) => match op {
            ConvertOp::I32WrapI64 => Some(Expr::i32_const(*v as i32)),
            ConvertOp::F32ConvertI64S => Some(Expr::f32_const(*v as f32)),
            ConvertOp::F32ConvertI64U => Some(Expr::f32_const(*v as u64 as f32)),
            ConvertOp::F64ConvertI64S => Some(Expr::f64_const(*v as f64)),
            ConvertOp::F64ConvertI64U => Some(Expr::f64_const(*v as u64 as f64)),
            ConvertOp::I64Extend8S => Some(Expr::i64_const(*v as i8 as i64)),
            ConvertOp::I64Extend16S => Some(Expr::i64_const(*v as i16 as i64)),
            ConvertOp::I64Extend32S => Some(Expr::i64_const(*v as i32 as i64)),
            ConvertOp::F64ReinterpretI64 => Some(Expr::f64_const(f64::from_bits(*v as u64))),
            _ => None,
        },

        ExprKind::F32Const(v) => match op {
            ConvertOp::F64PromoteF32 => Some(Expr::f64_const(*v as f64)),
            ConvertOp::I32TruncF32S => Some(Expr::i32_const(*v as i32)),
            ConvertOp::I32TruncF32U => Some(Expr::i32_const(*v as u32 as i32)),
            ConvertOp::I64TruncF32S => Some(Expr::i64_const(*v as i64)),
            ConvertOp::I64TruncF32U => Some(Expr::i64_const(*v as u64 as i64)),
            ConvertOp::I32ReinterpretF32 => Some(Expr::i32_const(v.to_bits() as i32)),
            _ => None,
        },

        ExprKind::F64Const(v) => match op {
            ConvertOp::F32DemoteF64 => Some(Expr::f32_const(*v as f32)),
            ConvertOp::I32TruncF64S => Some(Expr::i32_const(*v as i32)),
            ConvertOp::I32TruncF64U => Some(Expr::i32_const(*v as u32 as i32)),
            ConvertOp::I64TruncF64S => Some(Expr::i64_const(*v as i64)),
            ConvertOp::I64TruncF64U => Some(Expr::i64_const(*v as u64 as i64)),
            ConvertOp::I64ReinterpretF64 => Some(Expr::i64_const(v.to_bits() as i64)),
            _ => None,
        },

        _ => None,
    }
}

/// Check if a statement is dead (has no effect)
fn is_dead_stmt(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Nop => true,
        Stmt::Drop(expr) => {
            // Only dead if the expression has no side effects
            !has_side_effects(expr)
        }
        Stmt::Expr(expr) => !has_side_effects(expr),
        _ => false,
    }
}

/// Check if an expression has side effects
fn has_side_effects(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Call { .. } | ExprKind::CallIndirect { .. } => true,
        ExprKind::BinOp(_, a, b) => has_side_effects(a) || has_side_effects(b),
        ExprKind::UnaryOp(_, a) => has_side_effects(a),
        ExprKind::Compare(_, a, b, _) => has_side_effects(a) || has_side_effects(b),
        ExprKind::Load { addr, .. } => has_side_effects(addr),
        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => has_side_effects(cond) || has_side_effects(then_val) || has_side_effects(else_val),
        ExprKind::Convert { expr, .. } => has_side_effects(expr),
        ExprKind::GoString { ptr, len } => has_side_effects(ptr) || has_side_effects(len),
        ExprKind::GoSlice { ptr, len, cap } => {
            has_side_effects(ptr) || has_side_effects(len) || has_side_effects(cap)
        }
        ExprKind::GoInterface { type_ptr, data } => {
            has_side_effects(type_ptr) || has_side_effects(data)
        }
        _ => false,
    }
}
