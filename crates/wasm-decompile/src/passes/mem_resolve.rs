//! Memory resolution pass
//!
//! Resolves constant memory pointers to their string values when possible.

use crate::ir::*;
use crate::passes::{Pass, PassContext};

/// Memory resolution pass
pub struct MemResolvePass;

impl Pass for MemResolvePass {
    fn name(&self) -> &'static str {
        "mem_resolve"
    }

    fn run(&self, module: &mut Module, ctx: &mut PassContext) {
        // Clone memory to avoid borrow issues
        let memory = module.memory.clone();

        for func in &mut module.functions {
            if !func.is_import {
                resolve_block(&mut func.body, &memory, ctx);
            }
        }
    }
}

fn resolve_block(block: &mut Block, memory: &[u8], ctx: &mut PassContext) {
    for stmt in &mut block.stmts {
        resolve_stmt(stmt, memory, ctx);
    }
}

fn resolve_stmt(stmt: &mut Stmt, memory: &[u8], ctx: &mut PassContext) {
    match stmt {
        Stmt::LocalSet { value, .. } => {
            resolve_expr(value, memory, ctx);
        }
        Stmt::GlobalSet { value, .. } => {
            resolve_expr(value, memory, ctx);
        }
        Stmt::Store { addr, value, .. } => {
            resolve_expr(addr, memory, ctx);
            resolve_expr(value, memory, ctx);
        }
        Stmt::Expr(expr) => {
            resolve_expr(expr, memory, ctx);
        }
        Stmt::Return(Some(expr)) => {
            resolve_expr(expr, memory, ctx);
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            resolve_expr(cond, memory, ctx);
            resolve_block(then_block, memory, ctx);
            if let Some(else_blk) = else_block {
                resolve_block(else_blk, memory, ctx);
            }
        }
        Stmt::Block { body, .. } => {
            resolve_block(body, memory, ctx);
        }
        Stmt::Loop { body, .. } => {
            resolve_block(body, memory, ctx);
        }
        Stmt::DoWhile { body, cond } => {
            resolve_block(body, memory, ctx);
            resolve_expr(cond, memory, ctx);
        }
        Stmt::While { cond, body } => {
            resolve_expr(cond, memory, ctx);
            resolve_block(body, memory, ctx);
        }
        Stmt::BrIf { cond, .. } => {
            resolve_expr(cond, memory, ctx);
        }
        Stmt::BrTable { index, .. } => {
            resolve_expr(index, memory, ctx);
        }
        Stmt::Drop(expr) => {
            resolve_expr(expr, memory, ctx);
        }
        _ => {}
    }
}

fn resolve_expr(expr: &mut Expr, memory: &[u8], ctx: &mut PassContext) {
    match &mut expr.kind {
        ExprKind::Call { args, .. } => {
            // Try to resolve consecutive (ptr, len) arguments as strings
            let resolved = try_resolve_string_args(args, memory);
            if let Some(resolved_args) = resolved {
                *args = resolved_args;
            } else {
                // Regular resolve for each arg
                for arg in args {
                    resolve_expr(arg, memory, ctx);
                }
            }
        }

        ExprKind::CallIndirect { index, args, .. } => {
            resolve_expr(index, memory, ctx);
            for arg in args {
                resolve_expr(arg, memory, ctx);
            }
        }

        ExprKind::BinOp(_, a, b) => {
            resolve_expr(a, memory, ctx);
            resolve_expr(b, memory, ctx);
        }

        ExprKind::UnaryOp(_, a) => {
            resolve_expr(a, memory, ctx);
        }

        ExprKind::Compare(_, a, b) => {
            resolve_expr(a, memory, ctx);
            resolve_expr(b, memory, ctx);
        }

        ExprKind::Load { addr, .. } => {
            resolve_expr(addr, memory, ctx);
        }

        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => {
            resolve_expr(cond, memory, ctx);
            resolve_expr(then_val, memory, ctx);
            resolve_expr(else_val, memory, ctx);
        }

        ExprKind::Convert { expr: inner, .. } => {
            resolve_expr(inner, memory, ctx);
        }

        ExprKind::GoString { ptr, len } => {
            resolve_expr(ptr, memory, ctx);
            resolve_expr(len, memory, ctx);

            // If both ptr and len are constants, try to resolve
            if let (ExprKind::I32Const(p), ExprKind::I32Const(l)) = (&ptr.kind, &len.kind) {
                if let Some(s) = get_string_from_memory(memory, *p as usize, *l as usize) {
                    expr.kind = ExprKind::StringLiteral(s);
                    expr.ty = InferredType::GoString;
                }
            }
        }

        ExprKind::GoSlice { ptr, len, cap } => {
            resolve_expr(ptr, memory, ctx);
            resolve_expr(len, memory, ctx);
            resolve_expr(cap, memory, ctx);
        }

        ExprKind::GoInterface { type_ptr, data } => {
            resolve_expr(type_ptr, memory, ctx);
            resolve_expr(data, memory, ctx);
        }

        _ => {}
    }
}

/// Try to resolve consecutive (ptr, len) argument pairs as string literals
fn try_resolve_string_args(args: &Vec<Expr>, memory: &[u8]) -> Option<Vec<Expr>> {
    if args.len() < 2 {
        return None;
    }

    let mut resolved = Vec::new();
    let mut i = 0;
    let mut any_resolved = false;

    while i < args.len() {
        // Check for (ptr_const, len_const) pattern
        if i + 1 < args.len() {
            if let (ExprKind::I32Const(ptr), ExprKind::I32Const(len)) =
                (&args[i].kind, &args[i + 1].kind)
            {
                // Only try to resolve if this looks like a string pointer
                // (reasonable address range and length)
                if *ptr > 0 && *len > 0 && *len < 10000 {
                    if let Some(s) = get_string_from_memory(memory, *ptr as usize, *len as usize) {
                        // Successfully resolved - emit StringLiteral instead of ptr, len
                        resolved.push(Expr::with_type(
                            ExprKind::StringLiteral(s),
                            InferredType::GoString,
                        ));
                        i += 2;
                        any_resolved = true;
                        continue;
                    }
                }
            }
        }

        // Not a string pattern - keep original
        resolved.push(args[i].clone());
        i += 1;
    }

    if any_resolved {
        Some(resolved)
    } else {
        None
    }
}

/// Get a string from memory at the given offset and length
fn get_string_from_memory(memory: &[u8], offset: usize, len: usize) -> Option<String> {
    if offset + len > memory.len() {
        return None;
    }

    let bytes = &memory[offset..offset + len];

    // Check if it's valid UTF-8
    match std::str::from_utf8(bytes) {
        Ok(s) => {
            // Additional validation: should look like readable text
            if is_readable_string(s) {
                Some(s.to_string())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Check if a string looks like readable text
fn is_readable_string(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Count printable/readable characters
    let printable = s.chars().filter(|c| is_printable_char(*c)).count();
    let total = s.chars().count();

    // At least 70% should be printable
    printable * 100 / total >= 70
}

fn is_printable_char(c: char) -> bool {
    // Allow ASCII printable, newlines, tabs, and common unicode
    c.is_ascii_graphic()
        || c.is_ascii_whitespace()
        || c.is_alphabetic()
        || c.is_numeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_detection() {
        assert!(is_readable_string("hello world"));
        assert!(is_readable_string("Hello, World!"));
        assert!(is_readable_string("test\nwith\nnewlines"));
        assert!(!is_readable_string("\x00\x01\x02\x03"));
    }

    #[test]
    fn test_get_string_from_memory() {
        let memory = b"Hello, World!\x00Other stuff";
        assert_eq!(
            get_string_from_memory(memory, 0, 13),
            Some("Hello, World!".to_string())
        );
        assert_eq!(
            get_string_from_memory(memory, 0, 5),
            Some("Hello".to_string())
        );
        assert_eq!(get_string_from_memory(memory, 100, 5), None);
    }
}
