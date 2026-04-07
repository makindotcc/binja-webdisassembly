//! Go-specific passes
//!
//! These passes recognize patterns specific to Go-compiled WebAssembly.

mod string;

pub use string::GoStringPass;

use crate::ir::*;
use crate::passes::{Pass, PassContext};

/// Go slice pattern recognition pass
pub struct GoSlicePass;

impl Pass for GoSlicePass {
    fn name(&self) -> &'static str {
        "go_slice"
    }

    fn run(&self, module: &mut Module, ctx: &mut PassContext) {
        for func in &mut module.functions {
            if !func.is_import {
                transform_slice_patterns(&mut func.body, ctx);
            }
        }
    }
}

fn transform_slice_patterns(block: &mut Block, ctx: &mut PassContext) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Expr(expr) | Stmt::LocalSet { value: expr, .. } => {
                transform_slice_expr(expr, ctx);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                transform_slice_expr(cond, ctx);
                transform_slice_patterns(then_block, ctx);
                if let Some(else_blk) = else_block {
                    transform_slice_patterns(else_blk, ctx);
                }
            }
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                transform_slice_patterns(body, ctx);
            }
            _ => {}
        }
    }
}

fn transform_slice_expr(expr: &mut Expr, ctx: &mut PassContext) {
    match &mut expr.kind {
        ExprKind::Call { args, .. } => {
            // Try to detect (ptr, len, cap) triplets that represent slices
            let resolved = try_resolve_slice_args(args);
            if let Some(resolved_args) = resolved {
                *args = resolved_args;
            } else {
                for arg in args {
                    transform_slice_expr(arg, ctx);
                }
            }
        }

        ExprKind::BinOp(_, a, b) => {
            transform_slice_expr(a, ctx);
            transform_slice_expr(b, ctx);
        }

        ExprKind::UnaryOp(_, a) => {
            transform_slice_expr(a, ctx);
        }

        ExprKind::Compare(_, a, b, _) => {
            transform_slice_expr(a, ctx);
            transform_slice_expr(b, ctx);
        }

        ExprKind::Load { addr, .. } => {
            transform_slice_expr(addr, ctx);
        }

        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => {
            transform_slice_expr(cond, ctx);
            transform_slice_expr(then_val, ctx);
            transform_slice_expr(else_val, ctx);
        }

        ExprKind::Convert { expr: inner, .. } => {
            transform_slice_expr(inner, ctx);
        }

        _ => {}
    }
}

/// Try to resolve consecutive (ptr, len, cap) argument triplets as Go slices
fn try_resolve_slice_args(args: &Vec<Expr>) -> Option<Vec<Expr>> {
    if args.len() < 3 {
        return None;
    }

    let mut resolved = Vec::new();
    let mut i = 0;
    let mut any_resolved = false;

    while i < args.len() {
        // Check for (ptr, len, cap) pattern
        // In Go WASM, slices are typically passed as three i32 values
        if i + 2 < args.len() {
            let is_slice_pattern = matches!(
                (&args[i].ty, &args[i + 1].ty, &args[i + 2].ty),
                (InferredType::I32, InferredType::I32, InferredType::I32)
            );

            // Additional heuristic: len <= cap
            let looks_like_slice = if let (
                ExprKind::I32Const(_ptr),
                ExprKind::I32Const(len),
                ExprKind::I32Const(cap),
            ) = (&args[i].kind, &args[i + 1].kind, &args[i + 2].kind)
            {
                *len <= *cap && *cap > 0 && *cap < 1_000_000
            } else {
                // Could be dynamic values - use type heuristic
                is_slice_pattern
            };

            if looks_like_slice {
                // Create GoSlice expression
                resolved.push(Expr::with_type(
                    ExprKind::GoSlice {
                        ptr: Box::new(args[i].clone()),
                        len: Box::new(args[i + 1].clone()),
                        cap: Box::new(args[i + 2].clone()),
                    },
                    InferredType::GoSlice(Box::new(InferredType::Unknown)),
                ));
                i += 3;
                any_resolved = true;
                continue;
            }
        }

        resolved.push(args[i].clone());
        i += 1;
    }

    if any_resolved {
        Some(resolved)
    } else {
        None
    }
}

/// Go interface pattern recognition pass
pub struct GoInterfacePass;

impl Pass for GoInterfacePass {
    fn name(&self) -> &'static str {
        "go_interface"
    }

    fn run(&self, module: &mut Module, ctx: &mut PassContext) {
        for func in &mut module.functions {
            if !func.is_import {
                transform_interface_patterns(&mut func.body, ctx);
            }
        }
    }
}

fn transform_interface_patterns(block: &mut Block, ctx: &mut PassContext) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Expr(expr) | Stmt::LocalSet { value: expr, .. } => {
                transform_interface_expr(expr, ctx);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                transform_interface_expr(cond, ctx);
                transform_interface_patterns(then_block, ctx);
                if let Some(else_blk) = else_block {
                    transform_interface_patterns(else_blk, ctx);
                }
            }
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                transform_interface_patterns(body, ctx);
            }
            _ => {}
        }
    }
}

fn transform_interface_expr(expr: &mut Expr, ctx: &mut PassContext) {
    match &mut expr.kind {
        ExprKind::Call { args, .. } => {
            // Go interfaces are passed as (type_ptr, data_ptr) pairs
            let resolved = try_resolve_interface_args(args);
            if let Some(resolved_args) = resolved {
                *args = resolved_args;
            } else {
                for arg in args {
                    transform_interface_expr(arg, ctx);
                }
            }
        }

        ExprKind::BinOp(_, a, b) => {
            transform_interface_expr(a, ctx);
            transform_interface_expr(b, ctx);
        }

        ExprKind::UnaryOp(_, a) => {
            transform_interface_expr(a, ctx);
        }

        ExprKind::Compare(_, a, b, _) => {
            transform_interface_expr(a, ctx);
            transform_interface_expr(b, ctx);
        }

        ExprKind::Load { addr, .. } => {
            transform_interface_expr(addr, ctx);
        }

        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => {
            transform_interface_expr(cond, ctx);
            transform_interface_expr(then_val, ctx);
            transform_interface_expr(else_val, ctx);
        }

        ExprKind::Convert { expr: inner, .. } => {
            transform_interface_expr(inner, ctx);
        }

        _ => {}
    }
}

/// Try to resolve consecutive (type, data) argument pairs as Go interfaces
fn try_resolve_interface_args(_args: &Vec<Expr>) -> Option<Vec<Expr>> {
    // This is harder to detect without additional context
    // For now, we don't automatically convert - would need function signature hints
    None
}
