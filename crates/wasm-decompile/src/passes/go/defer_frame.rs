//! TinyGo defer frame pattern recognition pass
//!
//! TinyGo uses a linked list at a fixed global address (138276) for defer frames.
//! This pass collapses the repetitive setup/teardown into helper calls.

use crate::emit_js::JS_EMITTER;
use crate::ir::*;
use crate::passes::{HelperRegistry, Pass, PassContext};

const DEFER_CHAIN_ADDR: i32 = 138276;

const SETUP_FUNC_IDX: u32 = u32::MAX - 6;
const CLEANUP_FUNC_IDX: u32 = u32::MAX - 7;

pub const DEFER_SETUP: RuntimeHelperDecl = RuntimeHelperDecl("deferSetup");
pub const DEFER_CLEANUP: RuntimeHelperDecl = RuntimeHelperDecl("deferCleanup");

pub struct GoDeferPass;

impl Pass for GoDeferPass {
    fn name(&self) -> &'static str {
        "go_defer"
    }

    fn run(&self, module: &mut Module, _ctx: &mut PassContext) {
        let mut any_matched = false;
        for func in &mut module.functions {
            if !func.is_import {
                any_matched |= transform_defer(&mut func.body);
            }
        }
        if any_matched {
            module.functions.push(Function {
                index: SETUP_FUNC_IDX,
                name: Some("deferSetup".into()),
                params: vec![],
                results: vec![],
                locals: vec![],
                body: Block::new(),
                is_import: true,
            });
            module.functions.push(Function {
                index: CLEANUP_FUNC_IDX,
                name: Some("deferCleanup".into()),
                params: vec![],
                results: vec![],
                locals: vec![],
                body: Block::new(),
                is_import: true,
            });
            module.runtime_helpers.push(DEFER_SETUP);
            module.runtime_helpers.push(DEFER_CLEANUP);
        }
    }

    fn register_helpers(&self, registry: &mut HelperRegistry) {
        registry.register(DEFER_SETUP, JS_EMITTER, concat!(
            "function deferSetup(frameSize, nodeOffset) { ",
            "let f = (g0 - frameSize) | 0; g0 = f; ",
            "let prev = load_i32(138276); ",
            "store_i32(138276, (f + nodeOffset) | 0); ",
            "store_i32(f, prev, nodeOffset); ",
            "return prev; }",
        ).into());
        registry.register(DEFER_CLEANUP, JS_EMITTER, concat!(
            "function deferCleanup(prev, frameSize) { ",
            "store_i32(138276, prev); ",
            "g0 = (g0 + frameSize) | 0; }",
        ).into());
    }
}

fn transform_defer(block: &mut Block) -> bool {
    let mut matched = false;
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                matched |= transform_defer(body);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                matched |= transform_defer(then_block);
                if let Some(eb) = else_block {
                    matched |= transform_defer(eb);
                }
            }
            Stmt::Switch { cases, default, .. } => {
                for case in cases {
                    matched |= transform_defer(&mut case.body);
                }
                if let Some(def) = default {
                    matched |= transform_defer(def);
                }
            }
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                matched |= transform_defer(body);
                matched |= transform_defer(finally_block);
            }
            _ => {}
        }
    }

    matched |= collapse_prolog(block);
    matched |= collapse_epilog(block);
    matched
}

/// Match prolog pattern:
///   l_stack = (g0 - N) | 0;   // or g0 - N
///   g0 = l_stack;
///   ... (optional zero-init, defer ID stores) ...
///   l_prev = load_i32(138276);
///   store_i32(138276, l_stack + Z);
///   store_i32(l_stack, l_prev, Z);
fn collapse_prolog(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i + 3 < block.stmts.len() {
        if let Some(m) = try_match_prolog(&block.stmts, i) {
            // Remove matched stmts in reverse order to preserve indices
            for &idx in m.remove_indices.iter().rev() {
                block.stmts.remove(idx);
            }

            // Insert at the position of the first removed stmt
            let insert_at = m.remove_indices[0];

            // l_prev = deferSetup(frameSize, nodeOffset)
            block.stmts.insert(insert_at, Stmt::LocalSet {
                local: m.prev_local,
                value: Expr::new(ExprKind::Call {
                    func: SETUP_FUNC_IDX,
                    args: vec![
                        Expr::i32_const(m.frame_size),
                        Expr::i32_const(m.node_offset),
                    ],
                }),
            });
            // l_stack = g0 (deferSetup already updated g0)
            block.stmts.insert(insert_at + 1, Stmt::LocalSet {
                local: m.stack_local,
                value: Expr::global(0),
            });

            changed = true;
        }
        i += 1;
    }
    changed
}

/// Match epilog pattern:
///   store_i32(138276, l_prev);
///   g0 = (l_stack + N) | 0;
fn collapse_epilog(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i + 1 < block.stmts.len() {
        if let Some((prev_local, frame_size)) = try_match_epilog(&block.stmts, i) {
            block.stmts.remove(i);
            block.stmts.remove(i);

            block.stmts.insert(i, Stmt::Expr(
                Expr::new(ExprKind::Call {
                    func: CLEANUP_FUNC_IDX,
                    args: vec![
                        Expr::local(prev_local),
                        Expr::i32_const(frame_size),
                    ],
                }),
            ));
            changed = true;
        }
        i += 1;
    }
    changed
}

struct PrologMatch {
    stack_local: u32,
    prev_local: u32,
    frame_size: i32,
    node_offset: i32,
    /// Stmts between stack_alloc and load_prev
    middle_count: usize,
    /// Indices of all stmts to remove (sorted ascending)
    remove_indices: Vec<usize>,
}

fn try_match_prolog(stmts: &[Stmt], start: usize) -> Option<PrologMatch> {
    // Match: l_stack = (g0 - N) | 0
    let (stack_local, frame_size) = is_stack_alloc(&stmts[start])?;

    // Match: g0 = l_stack
    if !is_global_set_local(&stmts[start + 1], 0, stack_local) {
        return None;
    }

    // Scan forward for load_i32(138276) pattern
    let max_scan = 12.min(stmts.len() - start - 2);
    let mut found_prev = None;
    for j in 0..max_scan {
        let idx = start + 2 + j;
        if let Some(prev_local) = is_load_defer_chain(&stmts[idx]) {
            found_prev = Some((prev_local, j));
            break;
        }
    }
    let (prev_local, middle_count) = found_prev?;

    // After load_i32(138276), scan a small window (up to 4 stmts) for:
    // - store_i32(138276, ...) — the head push (required)
    // - store_i32(l_stack, l_prev, Z) — link store (optional)
    // - l_tmp = (l_stack + Z) | 0 — temp local (optional)
    // These can appear in any order.
    let scan_start = start + 2 + middle_count + 1;
    let scan_end = (scan_start + 4).min(stmts.len());

    let mut head_store_idx = None;
    let mut link_store_idx = None;
    let mut tmp_local_idx = None;
    let mut node_offset: Option<i32> = None;
    let mut tmp_local_id: Option<u32> = None;

    for j in scan_start..scan_end {
        // Check for direct head store: store_i32(138276, (l_stack+Z)|0) or store_i32(138276, l_stack)
        if head_store_idx.is_none() {
            if let Some(z) = is_store_defer_head_direct(&stmts[j], stack_local) {
                head_store_idx = Some(j);
                node_offset = Some(z);
                continue;
            }
        }
        // Check for store_i32(138276, l_tmp) where l_tmp is a known temp
        if head_store_idx.is_none() {
            if let Some(tmp) = tmp_local_id {
                if is_store_defer_head_local(&stmts[j], tmp) {
                    head_store_idx = Some(j);
                    continue;
                }
            }
        }
        // Check for l_tmp = (l_stack + Z) | 0
        if tmp_local_idx.is_none() {
            if let Some((tmp, z)) = is_local_add_const(&stmts[j], stack_local) {
                tmp_local_idx = Some(j);
                tmp_local_id = Some(tmp);
                if node_offset.is_none() {
                    node_offset = Some(z);
                }
                continue;
            }
        }
        // Check for store_i32(l_stack, l_prev, Z) — link store
        if link_store_idx.is_none() {
            if let Some(z) = node_offset {
                if is_store_link(&stmts[j], stack_local, prev_local, z) {
                    link_store_idx = Some(j);
                    continue;
                }
            }
            // Also try to detect link store when we don't know Z yet
            // store_i32(l_stack, l_prev, offset) → gives us Z
            if let Some(z) = is_store_link_any(&stmts[j], stack_local, prev_local) {
                link_store_idx = Some(j);
                if node_offset.is_none() {
                    node_offset = Some(z);
                }
                continue;
            }
        }
    }

    // Must have found the head store
    head_store_idx?;
    let node_offset = node_offset?;

    // Count how many stmts to remove after load_prev (between scan_start and the last matched stmt)
    let mut remove_indices: Vec<usize> = vec![
        start,                         // stack alloc
        start + 1,                     // g0 = l_stack
        start + 2 + middle_count,      // load_prev
    ];
    remove_indices.push(head_store_idx.unwrap());
    if let Some(idx) = link_store_idx {
        remove_indices.push(idx);
    }
    // Don't remove tmp_local_idx — the local may be used later in the function
    // Remove in reverse order to preserve indices
    remove_indices.sort();
    remove_indices.dedup();

    Some(PrologMatch {
        stack_local,
        prev_local,
        frame_size,
        node_offset,
        middle_count,
        remove_indices,
    })
}

/// Match: l = (g0 - N) | 0  OR  l = g0 - N
fn is_stack_alloc(stmt: &Stmt) -> Option<(u32, i32)> {
    if let Stmt::LocalSet { local, value } = stmt {
        // Pattern: (g0 - N) | 0  →  BinOp(Or, BinOp(Sub, Global(0), I32Const(N)), I32Const(0))
        // OR just: g0 - N  →  BinOp(Sub, Global(0), I32Const(N))
        if let ExprKind::BinOp(BinOp::Or, inner, zero) = &value.kind {
            if matches!(&zero.kind, ExprKind::I32Const(0)) {
                if let ExprKind::BinOp(BinOp::Sub, g0, n) = &inner.kind {
                    if matches!(&g0.kind, ExprKind::Global(0)) {
                        if let ExprKind::I32Const(size) = &n.kind {
                            return Some((*local, *size));
                        }
                    }
                }
            }
        }
        if let ExprKind::BinOp(BinOp::Sub, g0, n) = &value.kind {
            if matches!(&g0.kind, ExprKind::Global(0)) {
                if let ExprKind::I32Const(size) = &n.kind {
                    return Some((*local, *size));
                }
            }
        }
    }
    None
}

/// Match: g0 = l  (GlobalSet { global: g, value: Local(l) })
fn is_global_set_local(stmt: &Stmt, global: u32, local: u32) -> bool {
    matches!(stmt, Stmt::GlobalSet { global: g, value }
        if *g == global && matches!(&value.kind, ExprKind::Local(l) if *l == local))
}

/// Match: l = load_i32(138276)
fn is_load_defer_chain(stmt: &Stmt) -> Option<u32> {
    if let Stmt::LocalSet { local, value } = stmt {
        if let ExprKind::Load { addr, offset: 0, size: MemSize::I32, .. } = &value.kind {
            if matches!(&addr.kind, ExprKind::I32Const(DEFER_CHAIN_ADDR)) {
                return Some(*local);
            }
        }
    }
    None
}

/// Pattern A: store_i32(138276, (l_stack + Z) | 0) or store_i32(138276, l_stack)
fn is_store_defer_head_direct(stmt: &Stmt, stack_local: u32) -> Option<i32> {
    if let Stmt::Store { addr, offset: 0, value, size: MemSize::I32 } = stmt {
        if !matches!(&addr.kind, ExprKind::I32Const(DEFER_CHAIN_ADDR)) {
            return None;
        }
        // store_i32(138276, l_stack)  →  offset = 0
        if matches!(&value.kind, ExprKind::Local(l) if *l == stack_local) {
            return Some(0);
        }
        // store_i32(138276, (l_stack + Z) | 0)
        return extract_add_const(value, stack_local);
    }
    None
}

/// Pattern B: store_i32(138276, l_tmp) where l_tmp is a known local
fn is_store_defer_head_local(stmt: &Stmt, tmp_local: u32) -> bool {
    matches!(stmt, Stmt::Store { addr, offset: 0, value, size: MemSize::I32 }
        if matches!(&addr.kind, ExprKind::I32Const(DEFER_CHAIN_ADDR))
        && matches!(&value.kind, ExprKind::Local(l) if *l == tmp_local))
}

/// Match: l_tmp = (l_stack + Z) | 0  —  returns (tmp_local, Z)
fn is_local_add_const(stmt: &Stmt, stack_local: u32) -> Option<(u32, i32)> {
    if let Stmt::LocalSet { local, value } = stmt {
        let z = extract_add_const(value, stack_local)?;
        return Some((*local, z));
    }
    None
}

/// Extract Z from `(local + Z) | 0` or `local + Z`
fn extract_add_const(expr: &Expr, local: u32) -> Option<i32> {
    // (l + Z) | 0
    if let ExprKind::BinOp(BinOp::Or, inner, zero) = &expr.kind {
        if matches!(&zero.kind, ExprKind::I32Const(0)) {
            return extract_add_const_raw(inner, local);
        }
    }
    // l + Z
    extract_add_const_raw(expr, local)
}

fn extract_add_const_raw(expr: &Expr, local_idx: u32) -> Option<i32> {
    if let ExprKind::BinOp(BinOp::Add, l, r) = &expr.kind {
        if matches!(&l.kind, ExprKind::Local(l) if *l == local_idx) {
            if let ExprKind::I32Const(z) = &r.kind {
                return Some(*z);
            }
        }
    }
    // Also handle: l - (-Z) which TinyGo sometimes emits as Sub with negative
    if let ExprKind::BinOp(BinOp::Sub, l, r) = &expr.kind {
        if matches!(&l.kind, ExprKind::Local(l) if *l == local_idx) {
            if let ExprKind::I32Const(z) = &r.kind {
                return Some(-*z);
            }
        }
    }
    None
}

/// Match: store_i32(l_stack, l_prev, Z)
fn is_store_link(stmt: &Stmt, stack_local: u32, prev_local: u32, node_offset: i32) -> bool {
    if let Stmt::Store { addr, offset, value, size: MemSize::I32 } = stmt {
        if matches!(&addr.kind, ExprKind::Local(l) if *l == stack_local)
            && *offset == node_offset as u32
            && matches!(&value.kind, ExprKind::Local(l) if *l == prev_local)
        {
            return true;
        }
    }
    false
}

/// Match: store_i32(l_stack, l_prev, ANY_OFFSET) — returns offset
fn is_store_link_any(stmt: &Stmt, stack_local: u32, prev_local: u32) -> Option<i32> {
    if let Stmt::Store { addr, offset, value, size: MemSize::I32 } = stmt {
        if matches!(&addr.kind, ExprKind::Local(l) if *l == stack_local)
            && matches!(&value.kind, ExprKind::Local(l) if *l == prev_local)
        {
            return Some(*offset as i32);
        }
    }
    None
}

/// Match epilog: store_i32(138276, l_prev) followed by g0 = (l_stack + N) | 0
fn try_match_epilog(stmts: &[Stmt], start: usize) -> Option<(u32, i32)> {
    // store_i32(138276, l_prev)
    let prev_local = is_restore_defer_chain(&stmts[start])?;

    // g0 = (l_stack + N) | 0
    let frame_size = is_stack_free(&stmts[start + 1])?;

    Some((prev_local, frame_size))
}

/// Match: store_i32(138276, l_prev)  — Store { addr: I32Const(138276), value: Local(l), ... }
fn is_restore_defer_chain(stmt: &Stmt) -> Option<u32> {
    if let Stmt::Store { addr, offset: 0, value, size: MemSize::I32 } = stmt {
        if matches!(&addr.kind, ExprKind::I32Const(DEFER_CHAIN_ADDR)) {
            if let ExprKind::Local(l) = &value.kind {
                return Some(*l);
            }
        }
    }
    None
}

/// Match: g0 = (l_stack + N) | 0  OR  g0 = l_stack + N
fn is_stack_free(stmt: &Stmt) -> Option<i32> {
    if let Stmt::GlobalSet { global: 0, value } = stmt {
        // (l + N) | 0
        if let ExprKind::BinOp(BinOp::Or, inner, zero) = &value.kind {
            if matches!(&zero.kind, ExprKind::I32Const(0)) {
                if let ExprKind::BinOp(BinOp::Add, _local, n) = &inner.kind {
                    if let ExprKind::I32Const(size) = &n.kind {
                        return Some(*size);
                    }
                }
            }
        }
        // l + N
        if let ExprKind::BinOp(BinOp::Add, _local, n) = &value.kind {
            if let ExprKind::I32Const(size) = &n.kind {
                return Some(*size);
            }
        }
    }
    None
}
