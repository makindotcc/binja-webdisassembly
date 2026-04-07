//! Type inference pass
//!
//! Propagates type information through the IR to help with pattern recognition.

use crate::ir::*;
use crate::passes::{Pass, PassContext};

/// Type inference pass
pub struct TypeInferPass;

impl Pass for TypeInferPass {
    fn name(&self) -> &'static str {
        "type_infer"
    }

    fn run(&self, module: &mut Module, ctx: &mut PassContext) {
        for func in &mut module.functions {
            if !func.is_import {
                infer_function(func, ctx);
            }
        }
    }
}

fn infer_function(func: &mut Function, ctx: &mut PassContext) {
    let func_idx = func.index;

    // First, set up types for parameters
    for (i, param) in func.params.iter().enumerate() {
        let ty = match param {
            ValType::I32 => InferredType::I32,
            ValType::I64 => InferredType::I64,
            ValType::F32 => InferredType::F32,
            ValType::F64 => InferredType::F64,
        };
        ctx.var_types.insert((func_idx, i as u32), ty);
    }

    // Set up types for locals
    for (i, local) in func.locals.iter().enumerate() {
        let local_idx = func.params.len() as u32 + i as u32;
        let ty = match local {
            ValType::I32 => InferredType::I32,
            ValType::I64 => InferredType::I64,
            ValType::F32 => InferredType::F32,
            ValType::F64 => InferredType::F64,
        };
        ctx.var_types.insert((func_idx, local_idx), ty);
    }

    // Infer types in the function body
    infer_block(&mut func.body, func_idx, ctx);
}

fn infer_block(block: &mut Block, func_idx: u32, ctx: &mut PassContext) {
    for stmt in &mut block.stmts {
        infer_stmt(stmt, func_idx, ctx);
    }
}

fn infer_stmt(stmt: &mut Stmt, func_idx: u32, ctx: &mut PassContext) {
    match stmt {
        Stmt::LocalSet { local, value } => {
            infer_expr(value, ctx);
            // Update type from the assigned value
            if value.ty != InferredType::Unknown {
                ctx.var_types.insert((func_idx, *local), value.ty.clone());
            }
        }

        Stmt::GlobalSet { value, .. } => {
            infer_expr(value, ctx);
        }

        Stmt::Store { addr, value, .. } => {
            infer_expr(addr, ctx);
            infer_expr(value, ctx);
        }

        Stmt::Expr(expr) => {
            infer_expr(expr, ctx);
        }

        Stmt::Return(Some(expr)) => {
            infer_expr(expr, ctx);
        }

        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            infer_expr(cond, ctx);
            infer_block(then_block, func_idx, ctx);
            if let Some(else_blk) = else_block {
                infer_block(else_blk, func_idx, ctx);
            }
        }

        Stmt::Block { body, .. } => {
            infer_block(body, func_idx, ctx);
        }

        Stmt::Loop { body, .. } => {
            infer_block(body, func_idx, ctx);
        }

        Stmt::DoWhile { body, cond } => {
            infer_block(body, func_idx, ctx);
            infer_expr(cond, ctx);
        }

        Stmt::While { cond, body } => {
            infer_expr(cond, ctx);
            infer_block(body, func_idx, ctx);
        }

        Stmt::BrIf { cond, .. } => {
            infer_expr(cond, ctx);
        }

        Stmt::BrTable { index, .. } => {
            infer_expr(index, ctx);
        }

        Stmt::Drop(expr) => {
            infer_expr(expr, ctx);
        }

        _ => {}
    }
}

fn infer_expr(expr: &mut Expr, ctx: &mut PassContext) {
    match &mut expr.kind {
        ExprKind::I32Const(_) => {
            expr.ty = InferredType::I32;
        }
        ExprKind::I64Const(_) => {
            expr.ty = InferredType::I64;
        }
        ExprKind::F32Const(_) => {
            expr.ty = InferredType::F32;
        }
        ExprKind::F64Const(_) => {
            expr.ty = InferredType::F64;
        }

        ExprKind::Local(idx) => {
            // Look up inferred type from context
            // Since we don't have func_idx here, leave as Unknown unless already set
            if expr.ty == InferredType::Unknown {
                // Type might have been set during lifting based on ValType
                // Keep it if already set
            }
            // Try to look up from context - but we'd need func_idx
            // This is a limitation of the current design
            let _ = idx;
        }

        ExprKind::Global(_) => {
            // Similar limitation for globals
        }

        ExprKind::BinOp(op, a, b) => {
            infer_expr(a, ctx);
            infer_expr(b, ctx);

            // Result type depends on operation
            expr.ty = match op {
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::DivS
                | BinOp::DivU
                | BinOp::RemS
                | BinOp::RemU
                | BinOp::And
                | BinOp::Or
                | BinOp::Xor
                | BinOp::Shl
                | BinOp::ShrS
                | BinOp::ShrU
                | BinOp::Rotl
                | BinOp::Rotr => {
                    // Use type from operand
                    if a.ty.is_integer() {
                        a.ty.clone()
                    } else if b.ty.is_integer() {
                        b.ty.clone()
                    } else {
                        InferredType::I32
                    }
                }
                BinOp::FAdd
                | BinOp::FSub
                | BinOp::FMul
                | BinOp::FDiv
                | BinOp::FMin
                | BinOp::FMax
                | BinOp::FCopysign => {
                    if a.ty.is_float() {
                        a.ty.clone()
                    } else if b.ty.is_float() {
                        b.ty.clone()
                    } else {
                        InferredType::F64
                    }
                }
            };
        }

        ExprKind::UnaryOp(op, a) => {
            infer_expr(a, ctx);
            let inferred = match op {
                UnaryOp::Eqz => InferredType::Bool,
                UnaryOp::Clz | UnaryOp::Ctz | UnaryOp::Popcnt => a.ty.clone(),
                UnaryOp::FAbs
                | UnaryOp::FNeg
                | UnaryOp::FCeil
                | UnaryOp::FFloor
                | UnaryOp::FTrunc
                | UnaryOp::FNearest
                | UnaryOp::FSqrt => a.ty.clone(),
            };
            if inferred != InferredType::Unknown {
                expr.ty = inferred;
            }
        }

        ExprKind::Compare(_, a, b) => {
            infer_expr(a, ctx);
            infer_expr(b, ctx);
            expr.ty = InferredType::Bool;
        }

        ExprKind::Load { addr, size, .. } => {
            infer_expr(addr, ctx);
            expr.ty = match size {
                MemSize::I8 | MemSize::I16 | MemSize::I32 => InferredType::I32,
                MemSize::I64 => InferredType::I64,
                MemSize::F32 => InferredType::F32,
                MemSize::F64 => InferredType::F64,
            };
        }

        ExprKind::Call { args, .. } => {
            for arg in args {
                infer_expr(arg, ctx);
            }
            // Would need function type info to determine return type
        }

        ExprKind::CallIndirect { index, args, .. } => {
            infer_expr(index, ctx);
            for arg in args {
                infer_expr(arg, ctx);
            }
        }

        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => {
            infer_expr(cond, ctx);
            infer_expr(then_val, ctx);
            infer_expr(else_val, ctx);

            // Result type is type of operands
            if then_val.ty != InferredType::Unknown {
                expr.ty = then_val.ty.clone();
            } else if else_val.ty != InferredType::Unknown {
                expr.ty = else_val.ty.clone();
            }
        }

        ExprKind::Convert { op, expr: inner } => {
            infer_expr(inner, ctx);
            expr.ty = convert_result_type(*op);
        }

        ExprKind::StringLiteral(_) => {
            expr.ty = InferredType::CString;
        }

        ExprKind::GoString { ptr, len } => {
            infer_expr(ptr, ctx);
            infer_expr(len, ctx);
            expr.ty = InferredType::GoString;
        }

        ExprKind::GoSlice { ptr, len, cap } => {
            infer_expr(ptr, ctx);
            infer_expr(len, ctx);
            infer_expr(cap, ctx);
            // Type would need element type info
            expr.ty = InferredType::GoSlice(Box::new(InferredType::Unknown));
        }

        ExprKind::GoInterface { type_ptr, data } => {
            infer_expr(type_ptr, ctx);
            infer_expr(data, ctx);
            expr.ty = InferredType::GoInterface;
        }

        ExprKind::ResolvedPointer { .. } => {
            expr.ty = InferredType::CString;
        }
    }
}

fn convert_result_type(op: ConvertOp) -> InferredType {
    match op {
        ConvertOp::I32WrapI64
        | ConvertOp::I32TruncF32S
        | ConvertOp::I32TruncF32U
        | ConvertOp::I32TruncF64S
        | ConvertOp::I32TruncF64U
        | ConvertOp::I32ReinterpretF32
        | ConvertOp::I32Extend8S
        | ConvertOp::I32Extend16S
        | ConvertOp::I32TruncSatF32S
        | ConvertOp::I32TruncSatF32U
        | ConvertOp::I32TruncSatF64S
        | ConvertOp::I32TruncSatF64U => InferredType::I32,

        ConvertOp::I64ExtendI32S
        | ConvertOp::I64ExtendI32U
        | ConvertOp::I64TruncF32S
        | ConvertOp::I64TruncF32U
        | ConvertOp::I64TruncF64S
        | ConvertOp::I64TruncF64U
        | ConvertOp::I64ReinterpretF64
        | ConvertOp::I64Extend8S
        | ConvertOp::I64Extend16S
        | ConvertOp::I64Extend32S
        | ConvertOp::I64TruncSatF32S
        | ConvertOp::I64TruncSatF32U
        | ConvertOp::I64TruncSatF64S
        | ConvertOp::I64TruncSatF64U => InferredType::I64,

        ConvertOp::F32ConvertI32S
        | ConvertOp::F32ConvertI32U
        | ConvertOp::F32ConvertI64S
        | ConvertOp::F32ConvertI64U
        | ConvertOp::F32DemoteF64
        | ConvertOp::F32ReinterpretI32 => InferredType::F32,

        ConvertOp::F64ConvertI32S
        | ConvertOp::F64ConvertI32U
        | ConvertOp::F64ConvertI64S
        | ConvertOp::F64ConvertI64U
        | ConvertOp::F64PromoteF32
        | ConvertOp::F64ReinterpretI64 => InferredType::F64,
    }
}
