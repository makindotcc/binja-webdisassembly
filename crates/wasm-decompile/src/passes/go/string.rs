//! Go string pattern recognition
//!
//! Go strings in WASM are represented as (ptr, len) pairs.
//! This pass detects and transforms these patterns.

use crate::ir::*;
use crate::passes::{Pass, PassContext};

/// Go string pattern recognition pass
pub struct GoStringPass;

impl Pass for GoStringPass {
    fn name(&self) -> &'static str {
        "go_string"
    }

    fn run(&self, module: &mut Module, ctx: &mut PassContext) {
        for func in &mut module.functions {
            if !func.is_import {
                transform_string_patterns(&mut func.body, ctx);
            }
        }
    }
}

fn transform_string_patterns(block: &mut Block, ctx: &mut PassContext) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Expr(expr) => {
                transform_string_expr(expr, ctx);
            }
            Stmt::LocalSet { value, .. } => {
                transform_string_expr(value, ctx);
            }
            Stmt::GlobalSet { value, .. } => {
                transform_string_expr(value, ctx);
            }
            Stmt::Store { addr, value, .. } => {
                transform_string_expr(addr, ctx);
                transform_string_expr(value, ctx);
            }
            Stmt::Return(Some(expr)) => {
                transform_string_expr(expr, ctx);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                transform_string_expr(cond, ctx);
                transform_string_patterns(then_block, ctx);
                if let Some(else_blk) = else_block {
                    transform_string_patterns(else_blk, ctx);
                }
            }
            Stmt::Block { body, .. } => {
                transform_string_patterns(body, ctx);
            }
            Stmt::Loop { body, .. } => {
                transform_string_patterns(body, ctx);
            }
            Stmt::DoWhile { body, cond } => {
                transform_string_patterns(body, ctx);
                transform_string_expr(cond, ctx);
            }
            Stmt::While { cond, body } => {
                transform_string_expr(cond, ctx);
                transform_string_patterns(body, ctx);
            }
            Stmt::BrIf { cond, .. } => {
                transform_string_expr(cond, ctx);
            }
            Stmt::BrTable { index, .. } => {
                transform_string_expr(index, ctx);
            }
            Stmt::Drop(expr) => {
                transform_string_expr(expr, ctx);
            }
            _ => {}
        }
    }
}

fn transform_string_expr(expr: &mut Expr, ctx: &mut PassContext) {
    match &mut expr.kind {
        ExprKind::Call { func, args } => {
            // First, recursively transform args
            for arg in args.iter_mut() {
                transform_string_expr(arg, ctx);
            }

            // Then try to detect (ptr, len) patterns
            // This looks for consecutive i32 argument pairs where:
            // - First arg looks like a pointer (non-zero constant or variable)
            // - Second arg looks like a length (small positive constant or variable)
            let resolved = try_resolve_go_string_args(args);
            if let Some(resolved_args) = resolved {
                *args = resolved_args;
            }

            // Check if this is a known Go string function
            let _ = func; // Would use for function-specific patterns
        }

        ExprKind::CallIndirect { index, args, .. } => {
            transform_string_expr(index, ctx);
            for arg in args.iter_mut() {
                transform_string_expr(arg, ctx);
            }
        }

        ExprKind::BinOp(_, a, b) => {
            transform_string_expr(a, ctx);
            transform_string_expr(b, ctx);
        }

        ExprKind::UnaryOp(_, a) => {
            transform_string_expr(a, ctx);
        }

        ExprKind::Compare(_, a, b, _) => {
            transform_string_expr(a, ctx);
            transform_string_expr(b, ctx);
        }

        ExprKind::Load { addr, .. } => {
            transform_string_expr(addr, ctx);
        }

        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => {
            transform_string_expr(cond, ctx);
            transform_string_expr(then_val, ctx);
            transform_string_expr(else_val, ctx);
        }

        ExprKind::Convert { expr: inner, .. } => {
            transform_string_expr(inner, ctx);
        }

        _ => {}
    }
}

/// Try to resolve consecutive (ptr, len) argument pairs as Go strings
fn try_resolve_go_string_args(args: &Vec<Expr>) -> Option<Vec<Expr>> {
    if args.len() < 2 {
        return None;
    }

    let mut resolved = Vec::new();
    let mut i = 0;
    let mut any_resolved = false;

    while i < args.len() {
        // Check for (ptr, len) pattern
        if i + 1 < args.len() && looks_like_go_string_pair(&args[i], &args[i + 1]) {
            // Create GoString expression
            resolved.push(Expr::with_type(
                ExprKind::GoString {
                    ptr: Box::new(args[i].clone()),
                    len: Box::new(args[i + 1].clone()),
                },
                InferredType::GoString,
            ));
            i += 2;
            any_resolved = true;
            continue;
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

/// Check if two expressions look like a Go string (ptr, len) pair
fn looks_like_go_string_pair(ptr: &Expr, len: &Expr) -> bool {
    // Both should be i32 types
    let ptr_is_i32 = matches!(
        ptr.ty,
        InferredType::I32 | InferredType::Pointer(_) | InferredType::Unknown
    );
    let len_is_i32 = matches!(ptr.ty, InferredType::I32 | InferredType::Unknown);

    if !ptr_is_i32 || !len_is_i32 {
        return false;
    }

    // If both are constants, apply heuristics
    match (&ptr.kind, &len.kind) {
        (ExprKind::I32Const(p), ExprKind::I32Const(l)) => {
            // Pointer should be > 0 (valid address)
            // Length should be reasonable (> 0, < 10000)
            *p > 0 && *l > 0 && *l < 10000
        }

        // If ptr is constant and len is variable, still might be a string
        (ExprKind::I32Const(p), _) => *p > 0,

        // If both are variables, could be a string (can't tell for sure)
        (ExprKind::Local(_), ExprKind::Local(_)) => true,

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_string_detection() {
        // (1024, 5) looks like a string
        let ptr = Expr::i32_const(1024);
        let len = Expr::i32_const(5);
        assert!(looks_like_go_string_pair(&ptr, &len));

        // (0, 5) doesn't look like a string (null pointer)
        let ptr = Expr::i32_const(0);
        let len = Expr::i32_const(5);
        assert!(!looks_like_go_string_pair(&ptr, &len));

        // (1024, 0) doesn't look like a string (zero length)
        let ptr = Expr::i32_const(1024);
        let len = Expr::i32_const(0);
        assert!(!looks_like_go_string_pair(&ptr, &len));

        // (1024, 1000000) doesn't look like a string (too long)
        let ptr = Expr::i32_const(1024);
        let len = Expr::i32_const(1000000);
        assert!(!looks_like_go_string_pair(&ptr, &len));
    }
}
