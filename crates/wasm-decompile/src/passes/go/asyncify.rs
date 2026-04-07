//! Asyncify pattern recognition pass
//!
//! TinyGo uses asyncify to implement goroutines. This creates boilerplate
//! rewind/unwind sequences in every async-capable function.
//!
//! This pass collapses these sequences into calls to synthetic helper functions.

use crate::emit_js::JS_EMITTER;
use crate::ir::*;
use crate::passes::{HelperRegistry, Pass, PassContext};

const REWIND_FUNC_IDX: u32 = u32::MAX - 4;
const UNWIND_FUNC_IDX: u32 = u32::MAX - 5;

pub const ASYNCIFY_REWIND: RuntimeHelperDecl = RuntimeHelperDecl("asyncify_rewind");
pub const ASYNCIFY_UNWIND: RuntimeHelperDecl = RuntimeHelperDecl("asyncify_unwind");

pub struct AsyncifyPass;

impl Pass for AsyncifyPass {
    fn name(&self) -> &'static str {
        "go_asyncify"
    }

    fn run(&self, module: &mut Module, _ctx: &mut PassContext) {
        let mut any_matched = false;
        for func in &mut module.functions {
            if !func.is_import {
                any_matched |= transform_asyncify(&mut func.body);
            }
        }
        if any_matched {
            module.functions.push(Function {
                index: REWIND_FUNC_IDX,
                name: Some("asyncify_rewind".into()),
                params: vec![],
                results: vec![],
                locals: vec![],
                body: Block::new(),
                is_import: true,
            });
            module.functions.push(Function {
                index: UNWIND_FUNC_IDX,
                name: Some("asyncify_unwind".into()),
                params: vec![],
                results: vec![],
                locals: vec![],
                body: Block::new(),
                is_import: true,
            });
            module.runtime_helpers.push(ASYNCIFY_REWIND);
            module.runtime_helpers.push(ASYNCIFY_UNWIND);
        }
    }

    fn register_helpers(&self, registry: &mut HelperRegistry) {
        registry.register(
            ASYNCIFY_REWIND,
            JS_EMITTER,
            concat!(
                "function asyncify_rewind(size, ...layout) { ",
                "store_i32(g2, ((load_i32(g2) - size) | 0)); ",
                "let p = load_i32(g2), r = []; ",
                "for (let [t, o] of layout) { ",
                "if (t === \"i64\") r.push(load_i64(p, o)); ",
                "else r.push(load_i32(p, o)); ",
                "} return r; }",
            )
            .into(),
        );
        registry.register(
            ASYNCIFY_UNWIND,
            JS_EMITTER,
            concat!(
                "function asyncify_unwind(branchId, frameSize, ...layout) { ",
                "store_i32(load_i32(g2), branchId); ",
                "store_i32(g2, ((load_i32(g2) + 4) | 0)); ",
                "let p = load_i32(g2); ",
                "for (let [t, o, v] of layout) { ",
                "if (t === \"i64\") store_i64(p, v, o); ",
                "else store_i32(p, v, o); ",
                "} store_i32(g2, ((load_i32(g2) + frameSize) | 0)); }",
            )
            .into(),
        );
    }
}

fn transform_asyncify(block: &mut Block) -> bool {
    let mut matched = false;
    // Process children first (bottom-up)
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                matched |= transform_asyncify(body);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                matched |= transform_asyncify(then_block);
                if let Some(eb) = else_block {
                    matched |= transform_asyncify(eb);
                }
            }
            Stmt::Switch { cases, default, .. } => {
                for case in cases {
                    matched |= transform_asyncify(&mut case.body);
                }
                if let Some(def) = default {
                    matched |= transform_asyncify(def);
                }
            }
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                matched |= transform_asyncify(body);
                matched |= transform_asyncify(finally_block);
            }
            _ => {}
        }
    }

    let len_before = block.stmts.len();
    collapse_rewind(block);
    collapse_unwind(block);
    matched |= block.stmts.len() != len_before;
    matched
}

/// Build a layout pair: ["i32", offset] or ["i64", offset]
fn layout_pair(byte_size: u32, offset: u32) -> Expr {
    let ty = if byte_size == 8 { "i64" } else { "i32" };
    Expr::new(ExprKind::Array(vec![
        Expr::new(ExprKind::StringLiteral(ty.into())),
        Expr::i32_const(offset as i32),
    ]))
}

/// Build a layout triple: ["i32", offset, value] or ["i64", offset, value]
fn layout_triple(byte_size: u32, offset: u32, value: Expr) -> Expr {
    let ty = if byte_size == 8 { "i64" } else { "i32" };
    Expr::new(ExprKind::Array(vec![
        Expr::new(ExprKind::StringLiteral(ty.into())),
        Expr::i32_const(offset as i32),
        value,
    ]))
}

/// Collapse rewind into:
/// `[l0, l1, ...] = asyncify_rewind(frame_size, ["i32",0], ["i32",8], ...)`
fn collapse_rewind(block: &mut Block) {
    let mut i = 0;
    while i + 2 < block.stmts.len() {
        if let Some((count, frame_size, locals)) = try_match_rewind(&block.stmts, i) {
            for _ in 0..count {
                block.stmts.remove(i);
            }

            let mut args = vec![Expr::i32_const(frame_size as i32)];
            let mut local_ids = Vec::new();
            for (local, offset, byte_size) in &locals {
                local_ids.push(*local);
                args.push(layout_pair(*byte_size, *offset));
            }

            block.stmts.insert(
                i,
                Stmt::MultiAssign {
                    locals: local_ids,
                    value: Expr::new(ExprKind::Call {
                        func: REWIND_FUNC_IDX,
                        args,
                    }),
                },
            );
        }
        i += 1;
    }
}

/// Collapse unwind into:
/// `asyncify_unwind(branchId, frameSize, ["i32",0,l0], ["i32",8,l1], ...)`
fn collapse_unwind(block: &mut Block) {
    let mut i = 0;
    while i + 3 < block.stmts.len() {
        if let Some((count, branch_id, frame_size, locals)) = try_match_unwind(&block.stmts, i) {
            for _ in 0..count {
                block.stmts.remove(i);
            }

            let mut args = vec![branch_id, Expr::i32_const(frame_size as i32)];
            for (local, offset, byte_size) in &locals {
                args.push(layout_triple(*byte_size, *offset, Expr::local(*local)));
            }

            block.stmts.insert(
                i,
                Stmt::Expr(Expr::new(ExprKind::Call {
                    func: UNWIND_FUNC_IDX,
                    args,
                })),
            );
        }
        i += 1;
    }
}

/// Check if stmt is `store_i32(g2, load_i32(g2) - N)` and return N
fn is_g2_subtract(stmt: &Stmt) -> Option<u32> {
    if let Stmt::Store {
        addr,
        offset: 0,
        value,
        size: MemSize::I32,
    } = stmt
    {
        if !matches!(&addr.kind, ExprKind::Global(2)) {
            return None;
        }
        if let ExprKind::BinOp(BinOp::Sub, load_g2, n_expr) = &value.kind {
            if !is_load_global2(load_g2) {
                return None;
            }
            if let ExprKind::I32Const(n) = &n_expr.kind {
                return Some(*n as u32);
            }
        }
    }
    None
}

/// Check if stmt is `local = load_i32(g2)`
fn is_local_set_from_g2(stmt: &Stmt) -> Option<u32> {
    if let Stmt::LocalSet { local, value } = stmt {
        if is_load_global2(value) {
            return Some(*local);
        }
    }
    None
}

/// Check if stmt is `local = load_i32/i64(ptr_local, offset)`
fn is_local_set_from_ptr(stmt: &Stmt, ptr_local: u32) -> Option<(u32, u32, u32)> {
    if let Stmt::LocalSet { local, value } = stmt {
        if let ExprKind::Load {
            addr, offset, size, ..
        } = &value.kind
        {
            let byte_size = match size {
                MemSize::I32 | MemSize::F32 => 4,
                MemSize::I64 | MemSize::F64 => 8,
                _ => return None,
            };
            if matches!(&addr.kind, ExprKind::Local(l) if *l == ptr_local) {
                return Some((*local, *offset, byte_size));
            }
        }
    }
    None
}

fn is_load_global2(expr: &Expr) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Load { addr, offset: 0, size: MemSize::I32, .. }
        if matches!(&addr.kind, ExprKind::Global(2))
    )
}

fn try_match_rewind(stmts: &[Stmt], start: usize) -> Option<(usize, u32, Vec<(u32, u32, u32)>)> {
    let frame_size = is_g2_subtract(&stmts[start])?;
    let ptr_local = is_local_set_from_g2(&stmts[start + 1])?;

    let mut locals = Vec::new();
    let mut consumed_bytes: u32 = 0;
    let mut count = 2;

    for j in (start + 2)..stmts.len() {
        if let Some((local, offset, byte_size)) = is_local_set_from_ptr(&stmts[j], ptr_local) {
            locals.push((local, offset, byte_size));
            let end = offset + byte_size;
            if end > consumed_bytes {
                consumed_bytes = end;
            }
            count += 1;
        } else {
            break;
        }
    }

    if locals.is_empty() || consumed_bytes != frame_size {
        return None;
    }

    Some((count, frame_size, locals))
}

/// Check if stmt is `store_i32(load_i32(g2), value)`
fn is_store_to_g2_deref(stmt: &Stmt) -> Option<&Expr> {
    if let Stmt::Store {
        addr,
        offset: 0,
        value,
        size: MemSize::I32,
    } = stmt
    {
        if is_load_global2(addr) {
            return Some(value);
        }
    }
    None
}

/// Check if stmt is `store_i32(g2, load_i32(g2) + N)` and return N
fn is_g2_advance(stmt: &Stmt) -> Option<u32> {
    if let Stmt::Store {
        addr,
        offset: 0,
        value,
        size: MemSize::I32,
    } = stmt
    {
        if !matches!(&addr.kind, ExprKind::Global(2)) {
            return None;
        }
        if let ExprKind::BinOp(BinOp::Add, load_g2, n_expr) = &value.kind {
            if !is_load_global2(load_g2) {
                return None;
            }
            if let ExprKind::I32Const(n) = &n_expr.kind {
                return Some(*n as u32);
            }
        }
    }
    None
}

/// Check if stmt is `store(ptr_local, local_val, offset)`
fn is_store_to_ptr(stmt: &Stmt, ptr_local: u32) -> Option<(u32, u32, u32)> {
    if let Stmt::Store {
        addr,
        offset,
        value,
        size,
    } = stmt
    {
        let byte_size = match size {
            MemSize::I32 | MemSize::F32 => 4,
            MemSize::I64 | MemSize::F64 => 8,
            _ => return None,
        };
        if matches!(&addr.kind, ExprKind::Local(l) if *l == ptr_local) {
            if let ExprKind::Local(val_local) = &value.kind {
                return Some((*val_local, *offset, byte_size));
            }
        }
    }
    None
}

fn try_match_unwind(
    stmts: &[Stmt],
    start: usize,
) -> Option<(usize, Expr, u32, Vec<(u32, u32, u32)>)> {
    let branch_id = is_store_to_g2_deref(&stmts[start])?.clone();

    if is_g2_advance(&stmts[start + 1])? != 4 {
        return None;
    }

    let ptr_local = is_local_set_from_g2(&stmts[start + 2])?;

    let mut locals = Vec::new();
    let mut count = 3;

    for j in (start + 3)..stmts.len() {
        if let Some((val_local, offset, byte_size)) = is_store_to_ptr(&stmts[j], ptr_local) {
            locals.push((val_local, offset, byte_size));
            count += 1;
        } else {
            break;
        }
    }

    // Last stmt should be g2 advance by frame size
    let frame_size = if start + count < stmts.len() {
        if let Some(n) = is_g2_advance(&stmts[start + count]) {
            count += 1;
            n
        } else {
            return None;
        }
    } else {
        return None;
    };

    if locals.is_empty() {
        return None;
    }

    Some((count, branch_id, frame_size, locals))
}
