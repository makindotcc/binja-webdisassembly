//! Control flow recovery pass
//!
//! Recovers high-level control structures from WASM-style labeled blocks/loops.
//!
//! ## Patterns Recognized
//!
//! ### 1. Do-While Loop
//! ```text
//! loop $L:
//!     <body>
//!     br_if $L <cond>   ; continue if cond
//!     ; implicit break (fall through)
//! ```
//! Becomes: `do { body } while (cond);`
//!
//! ### 2. While Loop
//! ```text
//! block $B:
//!     loop $L:
//!         br_if $B (!cond)   ; break if !cond
//!         <body>
//!         br $L              ; continue
//! ```
//! Becomes: `while (cond) { body }`

use crate::ir::*;
use crate::passes::{Pass, PassContext};

pub struct ControlFlowPass;

impl Pass for ControlFlowPass {
    fn name(&self) -> &'static str {
        "control_flow"
    }

    fn run(&self, module: &mut Module, _ctx: &mut PassContext) {
        for func in &mut module.functions {
            if !func.is_import {
                recover_control_flow(&mut func.body);
            }
        }
    }
}

fn recover_control_flow(block: &mut Block) {
    // Recursively process nested structures first (bottom-up)
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Block { body, .. } | Stmt::Loop { body, .. } => {
                recover_control_flow(body);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                recover_control_flow(then_block);
                if let Some(eb) = else_block {
                    recover_control_flow(eb);
                }
            }
            Stmt::DoWhile { body, .. } | Stmt::While { body, .. } => {
                recover_control_flow(body);
            }
            Stmt::Switch {
                cases, default, ..
            } => {
                for case in cases {
                    recover_control_flow(&mut case.body);
                }
                if let Some(def) = default {
                    recover_control_flow(def);
                }
            }
            _ => {}
        }
    }

    // Apply transformations (order matters)
    recover_do_while(block);
    recover_while(block);
    recover_block_to_early_return(block);
    recover_block_to_unreachable(block);
    remove_unused_block_wrappers(block);
}

/// Pattern:
/// Loop { body..., BrIf(same_label, cond, is_loop=true) [, break] }
/// → DoWhile { body, cond }
fn recover_do_while(block: &mut Block) {
    let mut new_stmts = Vec::with_capacity(block.stmts.len());

    for stmt in std::mem::take(&mut block.stmts) {
        if let Stmt::Loop { label, mut body } = stmt {
            if let Some(transformed) = try_transform_to_do_while(label, &mut body) {
                new_stmts.push(transformed);
                continue;
            }
            new_stmts.push(Stmt::Loop { label, body });
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
}

fn try_transform_to_do_while(label: u32, body: &mut Block) -> Option<Stmt> {
    let len = body.stmts.len();
    if len == 0 {
        return None;
    }

    // Find the BrIf at the end (might be followed by break)
    let (br_if_idx, has_trailing_break) = if len >= 2 {
        match (&body.stmts[len - 2], &body.stmts[len - 1]) {
            (
                Stmt::BrIf {
                    label: l,
                    is_loop: true,
                    ..
                },
                Stmt::Br { is_loop: false, .. },
            ) if *l == label => (len - 2, true),
            _ => match &body.stmts[len - 1] {
                Stmt::BrIf {
                    label: l,
                    is_loop: true,
                    ..
                } if *l == label => (len - 1, false),
                _ => return None,
            },
        }
    } else {
        match &body.stmts[len - 1] {
            Stmt::BrIf {
                label: l,
                is_loop: true,
                ..
            } if *l == label => (len - 1, false),
            _ => return None,
        }
    };

    // Check no other references to this label in the body (except the BrIf we found)
    let body_without_brif = &body.stmts[..br_if_idx];
    if has_branch_to_label(body_without_brif, label) {
        return None;
    }

    // Extract condition
    let cond = if let Stmt::BrIf { cond, .. } = body.stmts.remove(br_if_idx) {
        cond
    } else {
        unreachable!()
    };

    // Remove trailing break if present
    if has_trailing_break && br_if_idx < body.stmts.len() {
        body.stmts.remove(br_if_idx);
    }

    Some(Stmt::DoWhile {
        body: std::mem::take(body),
        cond,
    })
}

/// Pattern:
/// Block { label: B, body: [
///     Loop { label: L, body: [
///         BrIf { label: B, cond: !cond, is_loop: false },
///         ...body...,
///         Br { label: L, is_loop: true }
///     ]}
/// ]}
/// → While { cond, body }
fn recover_while(block: &mut Block) {
    let mut new_stmts = Vec::with_capacity(block.stmts.len());

    for stmt in std::mem::take(&mut block.stmts) {
        if let Stmt::Block {
            label: block_label,
            body: mut block_body,
        } = stmt
        {
            if let Some(transformed) = try_transform_to_while(block_label, &mut block_body) {
                new_stmts.push(transformed);
                continue;
            }
            new_stmts.push(Stmt::Block {
                label: block_label,
                body: block_body,
            });
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
}

fn try_transform_to_while(block_label: u32, block_body: &mut Block) -> Option<Stmt> {
    // Block must contain exactly one statement: a Loop
    if block_body.stmts.len() != 1 {
        return None;
    }

    let loop_stmt = &mut block_body.stmts[0];
    let (loop_label, loop_body) = match loop_stmt {
        Stmt::Loop { label, body } => (*label, body),
        _ => return None,
    };

    // Loop body must have at least 2 statements: BrIf + Br
    if loop_body.stmts.len() < 2 {
        return None;
    }

    // First statement must be BrIf to block_label (not loop)
    let cond = match &loop_body.stmts[0] {
        Stmt::BrIf {
            label,
            cond,
            is_loop: false,
        } if *label == block_label => cond.clone(),
        _ => return None,
    };

    // Last statement must be Br to loop_label (is_loop)
    let last_idx = loop_body.stmts.len() - 1;
    match &loop_body.stmts[last_idx] {
        Stmt::Br {
            label,
            is_loop: true,
        } if *label == loop_label => {}
        _ => return None,
    }

    // Check that the body (middle part) doesn't reference block_label or loop_label
    let middle = &loop_body.stmts[1..last_idx];
    if has_branch_to_label(middle, block_label) || has_branch_to_label(middle, loop_label) {
        return None;
    }

    // Extract the body (middle statements)
    let body_stmts: Vec<Stmt> = loop_body.stmts.drain(1..last_idx).collect();

    // Negate the condition (BrIf breaks on !cond, so while loop continues on cond)
    let while_cond = negate_condition(cond);

    // Recursively process the extracted body
    let mut while_body = Block::with_stmts(body_stmts);
    recover_control_flow(&mut while_body);

    Some(Stmt::While {
        cond: while_cond,
        body: while_body,
    })
}

/// Check if any statement references the given label (Br/BrIf only, not BrTable)
fn has_simple_branch_to_label(stmts: &[Stmt], label: u32) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Br { label: l, .. } | Stmt::BrIf { label: l, .. } if *l == label => {
                return true;
            }
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                if has_simple_branch_to_label(&body.stmts, label) {
                    return true;
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if has_simple_branch_to_label(&then_block.stmts, label) {
                    return true;
                }
                if let Some(eb) = else_block {
                    if has_simple_branch_to_label(&eb.stmts, label) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Check if any BrTable references the given label (can't be safely replaced)
fn has_br_table_to_label(stmts: &[Stmt], label: u32) -> bool {
    for stmt in stmts {
        match stmt {
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
                if has_br_table_to_label(&body.stmts, label) {
                    return true;
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if has_br_table_to_label(&then_block.stmts, label) {
                    return true;
                }
                if let Some(eb) = else_block {
                    if has_br_table_to_label(&eb.stmts, label) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Check if any statement references the given label (includes BrTable)
fn has_branch_to_label(stmts: &[Stmt], label: u32) -> bool {
    has_simple_branch_to_label(stmts, label) || has_br_table_to_label(stmts, label)
}

/// Pattern 1: Block { label: L, body } followed by Return(X)
/// → Replace all `break L` with `return X`, inline block
///
/// Pattern 2: Block { label: L, body } + LocalSet { local, value } + Return(Local(local))
/// → Replace all `break L` with `return value`, inline block
///
/// Example (Pattern 1):
/// ```text
/// block_0: {
///   if (cond) break block_0;
///   ...code...
/// }
/// return result;
/// ```
/// Becomes:
/// ```text
/// if (cond) return result;
/// ...code...
/// return result;
/// ```
///
/// Example (Pattern 2):
/// ```text
/// block_0: {
///   if (cond) break block_0;
///   ...code...
/// }
/// l0 = 0;
/// return l0;
/// ```
/// Becomes:
/// ```text
/// if (cond) return 0;
/// ...code...
/// l0 = 0;
/// return l0;
/// ```
fn recover_block_to_early_return(block: &mut Block) {
    let mut i = 0;
    while i < block.stmts.len() {
        // Pattern 2: Block + LocalSet + Return(Local)
        if i + 2 < block.stmts.len() {
            let (is_pattern2, has_br_table) = if let (
                Stmt::Block { label, body },
                Stmt::LocalSet {
                    local: set_local, ..
                },
                Stmt::Return(Some(ret_expr)),
            ) =
                (&block.stmts[i], &block.stmts[i + 1], &block.stmts[i + 2])
            {
                let matches_pattern =
                    matches!(&ret_expr.kind, ExprKind::Local(l) if *l == *set_local);
                let has_br_table = has_br_table_to_label(&body.stmts, *label);
                (matches_pattern, has_br_table)
            } else {
                (false, false)
            };

            // Skip if BrTable references this label (can't safely replace)
            if is_pattern2 && !has_br_table {
                // Extract pieces
                let block_stmt = block.stmts.remove(i);
                // LocalSet is now at i, Return at i+1

                if let Stmt::Block { label, mut body } = block_stmt {
                    // Get the value from LocalSet
                    let ret_val = if let Stmt::LocalSet { value, .. } = &block.stmts[i] {
                        value.clone()
                    } else {
                        unreachable!()
                    };

                    // Replace breaks with return value
                    replace_breaks_with_return(&mut body, label, &ret_val);

                    // Insert block body before LocalSet
                    for (j, stmt) in body.stmts.into_iter().enumerate() {
                        block.stmts.insert(i + j, stmt);
                    }
                    continue;
                }
            }
        }

        // Pattern 1: Block + Return
        if i + 1 < block.stmts.len() {
            let (is_pattern1, has_br_table) =
                if let (Stmt::Block { label, body }, Stmt::Return(Some(_))) =
                    (&block.stmts[i], &block.stmts[i + 1])
                {
                    (true, has_br_table_to_label(&body.stmts, *label))
                } else {
                    (false, false)
                };

            // Skip if BrTable references this label
            if is_pattern1 && !has_br_table {
                let block_stmt = block.stmts.remove(i);
                let return_stmt = block.stmts[i].clone();

                if let Stmt::Block { label, mut body } = block_stmt {
                    if let Stmt::Return(Some(ret_val)) = &return_stmt {
                        replace_breaks_with_return(&mut body, label, ret_val);

                        for (j, stmt) in body.stmts.into_iter().enumerate() {
                            block.stmts.insert(i + j, stmt);
                        }
                        continue;
                    }
                }
            }
        }

        i += 1;
    }
}

/// Replace `Br { label }` and `BrIf { label, cond }` with return statements
fn replace_breaks_with_return(block: &mut Block, target_label: u32, ret_val: &Expr) {
    let stmts = std::mem::take(&mut block.stmts);
    let mut new_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts {
        match stmt {
            // br label -> return X
            Stmt::Br {
                label,
                is_loop: false,
            } if label == target_label => {
                new_stmts.push(Stmt::Return(Some(ret_val.clone())));
            }
            // br_if label, cond -> if (cond) return X
            Stmt::BrIf {
                label,
                cond,
                is_loop: false,
            } if label == target_label => {
                new_stmts.push(Stmt::If {
                    cond,
                    then_block: Block::with_stmts(vec![Stmt::Return(Some(ret_val.clone()))]),
                    else_block: None,
                });
            }
            // Recurse into nested structures
            Stmt::Block { label, mut body } => {
                replace_breaks_with_return(&mut body, target_label, ret_val);
                new_stmts.push(Stmt::Block { label, body });
            }
            Stmt::Loop { label, mut body } => {
                replace_breaks_with_return(&mut body, target_label, ret_val);
                new_stmts.push(Stmt::Loop { label, body });
            }
            Stmt::If {
                cond,
                mut then_block,
                mut else_block,
            } => {
                replace_breaks_with_return(&mut then_block, target_label, ret_val);
                if let Some(ref mut eb) = else_block {
                    replace_breaks_with_return(eb, target_label, ret_val);
                }
                new_stmts.push(Stmt::If {
                    cond,
                    then_block,
                    else_block,
                });
            }
            Stmt::DoWhile { mut body, cond } => {
                replace_breaks_with_return(&mut body, target_label, ret_val);
                new_stmts.push(Stmt::DoWhile { body, cond });
            }
            Stmt::While { cond, mut body } => {
                replace_breaks_with_return(&mut body, target_label, ret_val);
                new_stmts.push(Stmt::While { cond, body });
            }
            other => new_stmts.push(other),
        }
    }

    block.stmts = new_stmts;
}

/// Pattern: Block { label: L, body } followed by Unreachable
/// → Replace all `break L` with Unreachable, inline block
///
/// Example:
/// ```text
/// block_0: {
///   if (cond) break block_0;
///   ...code...
/// }
/// throw new Error('unreachable');
/// ```
/// Becomes:
/// ```text
/// if (cond) throw new Error('unreachable');
/// ...code...
/// throw new Error('unreachable');
/// ```
fn recover_block_to_unreachable(block: &mut Block) {
    let mut i = 0;
    while i + 1 < block.stmts.len() {
        let (is_match, has_br_table) = if let (Stmt::Block { label, body }, Stmt::Unreachable) =
            (&block.stmts[i], &block.stmts[i + 1])
        {
            (true, has_br_table_to_label(&body.stmts, *label))
        } else {
            (false, false)
        };

        // Skip if BrTable references this label
        if is_match && !has_br_table {
            let block_stmt = block.stmts.remove(i);
            // Unreachable is now at index i

            if let Stmt::Block { label, mut body } = block_stmt {
                // Replace breaks with unreachable
                replace_breaks_with_unreachable(&mut body, label);

                // Insert block body before Unreachable
                for (j, stmt) in body.stmts.into_iter().enumerate() {
                    block.stmts.insert(i + j, stmt);
                }
                continue;
            }
        }
        i += 1;
    }
}

/// Replace `Br { label }` and `BrIf { label, cond }` with Unreachable statements
fn replace_breaks_with_unreachable(block: &mut Block, target_label: u32) {
    let stmts = std::mem::take(&mut block.stmts);
    let mut new_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts {
        match stmt {
            // br label -> unreachable
            Stmt::Br {
                label,
                is_loop: false,
            } if label == target_label => {
                new_stmts.push(Stmt::Unreachable);
            }
            // br_if label, cond -> if (cond) unreachable
            Stmt::BrIf {
                label,
                cond,
                is_loop: false,
            } if label == target_label => {
                new_stmts.push(Stmt::If {
                    cond,
                    then_block: Block::with_stmts(vec![Stmt::Unreachable]),
                    else_block: None,
                });
            }
            // Recurse into nested structures
            Stmt::Block { label, mut body } => {
                replace_breaks_with_unreachable(&mut body, target_label);
                new_stmts.push(Stmt::Block { label, body });
            }
            Stmt::Loop { label, mut body } => {
                replace_breaks_with_unreachable(&mut body, target_label);
                new_stmts.push(Stmt::Loop { label, body });
            }
            Stmt::If {
                cond,
                mut then_block,
                mut else_block,
            } => {
                replace_breaks_with_unreachable(&mut then_block, target_label);
                if let Some(ref mut eb) = else_block {
                    replace_breaks_with_unreachable(eb, target_label);
                }
                new_stmts.push(Stmt::If {
                    cond,
                    then_block,
                    else_block,
                });
            }
            Stmt::DoWhile { mut body, cond } => {
                replace_breaks_with_unreachable(&mut body, target_label);
                new_stmts.push(Stmt::DoWhile { body, cond });
            }
            Stmt::While { cond, mut body } => {
                replace_breaks_with_unreachable(&mut body, target_label);
                new_stmts.push(Stmt::While { cond, body });
            }
            other => new_stmts.push(other),
        }
    }

    block.stmts = new_stmts;
}

/// Remove unused block wrappers
///
/// If a Block has no breaks targeting its label, inline the body.
/// This removes redundant block_N: { ... } wrappers.
fn remove_unused_block_wrappers(block: &mut Block) {
    // First, collect all labels that are referenced by BrTable in this block
    // (since BrTable can reference sibling blocks)
    let mut br_table_labels = std::collections::HashSet::new();
    collect_br_table_labels(&block.stmts, &mut br_table_labels);

    let stmts = std::mem::take(&mut block.stmts);
    let mut new_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts {
        if let Stmt::Block { label, body } = stmt {
            // Check if any statement in the body references this label
            // OR if any BrTable in the parent block references it
            if !has_branch_to_label(&body.stmts, label) && !br_table_labels.contains(&label) {
                // No breaks to this label - inline the body
                new_stmts.extend(body.stmts);
            } else {
                // Keep the block
                new_stmts.push(Stmt::Block { label, body });
            }
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
}

/// Collect all labels referenced by BrTable statements
fn collect_br_table_labels(stmts: &[Stmt], labels: &mut std::collections::HashSet<u32>) {
    for stmt in stmts {
        match stmt {
            Stmt::BrTable {
                targets, default, ..
            } => {
                for target in targets {
                    labels.insert(target.label);
                }
                labels.insert(default.label);
            }
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::While { body, .. }
            | Stmt::DoWhile { body, .. } => {
                collect_br_table_labels(&body.stmts, labels);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_br_table_labels(&then_block.stmts, labels);
                if let Some(eb) = else_block {
                    collect_br_table_labels(&eb.stmts, labels);
                }
            }
            _ => {}
        }
    }
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
            Expr::with_type(ExprKind::Compare(negated_op, a, b, operand_ty), InferredType::Bool)
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
