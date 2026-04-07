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

use std::collections::{HashMap, HashSet};

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
                if func.results.is_empty() {
                    recover_void_block(&mut func.body);
                    recover_void_epilog(&mut func.body);
                }
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
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                recover_control_flow(body);
                recover_control_flow(finally_block);
            }
            _ => {}
        }
    }

    // Apply transformations iteratively until no more changes
    let mut changed = true;
    let mut iterations = 0;
    while changed && iterations < 50 {
        changed = false;
        iterations += 1;
        changed |= recover_switch(block);
        changed |= recover_do_while(block);
        changed |= recover_while(block);
        changed |= recover_if_else(block);
        changed |= recover_block_with_single_break(block);
        changed |= recover_block_ending_with_break(block);
        changed |= recover_block_to_early_return(block);
        // changed |= recover_block_with_terminal_body(block);
        changed |= recover_block_to_unreachable(block);
        changed |= remove_unused_block_wrappers(block);
    }
}

/// Recover switch statements from br_table + nested blocks pattern.
///
/// The WASM pattern is:
/// ```text
/// Block { label: L0, body: [
///   Block { label: L1, body: [
///     Block { label: L2, body: [
///       ...
///         Block { label: LN, body: [
///           BrTable { index, targets: [LN, LN-1, ...], default: L0 }
///         ]},
///         ...case_for_LN (tail after Block LN)...
///       ]},
///       ...case_for_L2 (tail)...
///     ]},
///     ...case_for_L1 (tail)...
///   ]}
/// ]}
/// ...default_body (stmts after Block L0 in parent)...
/// ```
///
/// Breaking to a Block label jumps to the END of that block, so the tail
/// after each inner block is the case body for whoever targets that block's label.
fn recover_switch(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < block.stmts.len() {
        if let Some((switch_stmt, default_body, consumed)) = try_extract_switch(block, i) {
            // Remove the block stmt (and any default stmts consumed)
            block.stmts.remove(i);
            for _ in 0..consumed {
                block.stmts.remove(i);
            }
            // Insert default body stmts if any, then the switch
            if let Some(def) = &default_body {
                // default is embedded in the switch
                let _ = def;
            }
            block.stmts.insert(i, switch_stmt);
            changed = true;
        }
        i += 1;
    }
    changed
}

/// Try to extract a switch statement starting at block.stmts[idx].
/// Returns (Switch stmt, default body, number of stmts consumed after the block for default).
fn try_extract_switch(block: &Block, idx: usize) -> Option<(Stmt, Option<Block>, usize)> {
    // The outermost block
    let outer_block = match &block.stmts[idx] {
        Stmt::Block { label, body } => (*label, body),
        _ => return None,
    };
    let outer_label = outer_block.0;
    let outer_body = outer_block.1;

    // Peel the nested block chain to find the BrTable at the core.
    // We collect: (label, tail_stmts) for each block in the chain.
    // The chain is: each block body starts with another block, followed by tail stmts.
    let mut chain_labels: Vec<u32> = Vec::new();
    let mut chain_tails: Vec<Vec<Stmt>> = Vec::new();

    chain_labels.push(outer_label);

    let mut current_body = outer_body;

    loop {
        if current_body.stmts.is_empty() {
            return None;
        }

        // First statement should be a Block
        match &current_body.stmts[0] {
            Stmt::Block { label, body } => {
                let inner_label = *label;
                let tail: Vec<Stmt> = current_body.stmts[1..].to_vec();

                chain_labels.push(inner_label);
                chain_tails.push(tail);
                current_body = body;
            }
            _ => {
                // This is the innermost level - should contain BrTable
                break;
            }
        }
    }

    // The innermost body should contain a BrTable (possibly preceded by some setup code)
    let br_table_idx = current_body
        .stmts
        .iter()
        .position(|s| matches!(s, Stmt::BrTable { .. }))?;

    let (index_expr, targets, default_target) = match &current_body.stmts[br_table_idx] {
        Stmt::BrTable {
            index,
            targets,
            default,
        } => (index.clone(), targets.clone(), *default),
        _ => unreachable!(),
    };

    // Collect any pre-BrTable stmts (setup code before the br_table)
    let pre_br_table: Vec<Stmt> = current_body.stmts[..br_table_idx].to_vec();

    // All labels in the chain (outer to inner)
    // chain_labels = [L0, L1, L2, ..., LN]
    // chain_tails = [tail_for_L1, tail_for_L2, ..., tail_for_LN]
    // Note: chain_tails[i] is the tail after the block labeled chain_labels[i+1],
    //       which means it's the case body for targets pointing to chain_labels[i+1].

    // Build a map: label -> index in chain
    let label_to_chain_idx: HashMap<u32, usize> = chain_labels
        .iter()
        .enumerate()
        .map(|(i, &l)| (l, i))
        .collect();

    // Check: all br_table targets must point to labels in our chain
    for target in &targets {
        if !label_to_chain_idx.contains_key(&target.label) {
            return None;
        }
    }
    if !label_to_chain_idx.contains_key(&default_target.label) {
        return None;
    }

    // Group case values by target label
    let mut label_cases: HashMap<u32, Vec<u32>> = HashMap::new();
    for (case_val, target) in targets.iter().enumerate() {
        label_cases
            .entry(target.label)
            .or_default()
            .push(case_val as u32);
    }

    // Identify which label is the default
    let default_label = default_target.label;

    // Determine the "intermediate" chain labels: labels that are part of the nesting
    // chain but are NOT the outermost label. Case bodies must not reference these.
    let intermediate_labels: HashSet<u32> = chain_labels.iter().copied().collect();

    // Build switch cases from chain_tails
    // chain_labels[0] = outer_label (L0), the default target typically
    // chain_tails[0] = tail after block L1 = case body for targets pointing to L1
    // chain_tails[k] = tail after block L_{k+1} = case body for targets pointing to L_{k+1}
    let mut cases: Vec<SwitchCase> = Vec::new();

    for (tail_idx, tail) in chain_tails.iter().enumerate() {
        let target_label = chain_labels[tail_idx + 1]; // The label this tail belongs to

        // Get case values for this target label
        let values = match label_cases.get(&target_label) {
            Some(v) => v.clone(),
            None => {
                // No br_table entries point here - check if default points here
                if default_label == target_label {
                    // This is handled as the default case
                    continue;
                }
                // No case values and not default - skip (empty case)
                continue;
            }
        };

        if values.is_empty() && default_label != target_label {
            continue;
        }

        let mut case_body = tail.clone();

        // Safety: bail out if case body references any intermediate chain label
        // (except the outermost which we'll remap to u32::MAX for break)
        for &label in &intermediate_labels {
            if label == outer_label {
                continue; // outer_label refs become unqualified breaks
            }
            if has_branch_to_label(&case_body, label) {
                return None;
            }
        }

        // Remove trailing Br { label: outer_label } (becomes implicit break)
        if let Some(last) = case_body.last() {
            if matches!(last, Stmt::Br { label, is_loop: false } if *label == outer_label) {
                case_body.pop();
            }
        }

        // Replace mid-body Br/BrIf targeting outer_label with u32::MAX (unqualified break)
        replace_break_label(&mut case_body, outer_label, u32::MAX);

        cases.push(SwitchCase {
            values,
            body: Block::with_stmts(case_body),
        });
    }

    // Handle case values targeting outer_label.
    // The outer_label has no chain_tail — its "tail" is the stmts after the outer block in parent.
    let parent_tail: Vec<Stmt> = block.stmts[(idx + 1)..].to_vec();
    let outer_case_values = label_cases.get(&outer_label).cloned().unwrap_or_default();
    let mut consumed_tail = 0;

    // Check if any chain_tail falls through (doesn't end with a terminator).
    // If so, parent_tail is shared exit code reachable by both fallthrough
    // and break-to-outer — it must NOT be consumed as a case body.
    let any_falls_through = chain_tails.iter().any(|tail| {
        !matches!(
            tail.last(),
            Some(Stmt::Br { .. })
                | Some(Stmt::Return(_))
                | Some(Stmt::Unreachable)
                | Some(Stmt::BrTable { .. })
        )
    });

    if !outer_case_values.is_empty() {
        // If any case falls through to parent_tail, it's shared code — bail out
        if any_falls_through && !parent_tail.is_empty() {
            return None;
        }
        // Safety: check parent tail doesn't reference intermediate labels
        let mut safe = true;
        for &label in &intermediate_labels {
            if label == outer_label {
                continue;
            }
            if has_branch_to_label(&parent_tail, label) {
                safe = false;
                break;
            }
        }
        if !safe {
            return None;
        }

        let case_body = parent_tail.clone();
        consumed_tail = case_body.len();
        cases.push(SwitchCase {
            values: outer_case_values,
            body: Block::with_stmts(case_body),
        });
    }

    // Handle pre-br_table stmts: if there are setup stmts, we can't represent them cleanly.
    // For now, only handle the simple case where there are no pre-br_table stmts.
    if !pre_br_table.is_empty() {
        return None;
    }

    // Build default body
    // If default_label == outer_label, the default falls through to stmts after the outer block.
    // Those stmts are shared exit code reachable from any case that breaks, so they must stay
    // after the switch — not be consumed as default body.
    let (default_block, consumed) = if default_label == outer_label {
        // If we already consumed the tail for explicit cases, report that
        (None, consumed_tail)
    } else {
        // Default points to an intermediate label - get its tail
        let chain_idx = label_to_chain_idx[&default_label];
        if chain_idx > 0 {
            // The tail for this label is chain_tails[chain_idx - 1]
            let mut default_stmts = chain_tails[chain_idx - 1].clone();

            // Safety check: default body must not reference intermediate labels
            for &label in &intermediate_labels {
                if label == outer_label {
                    continue;
                }
                if has_branch_to_label(&default_stmts, label) {
                    return None;
                }
            }

            // Remove trailing Br to outer_label
            if let Some(last) = default_stmts.last() {
                if matches!(last, Stmt::Br { label, is_loop: false } if *label == outer_label) {
                    default_stmts.pop();
                }
            }
            replace_break_label(&mut default_stmts, outer_label, u32::MAX);

            if default_stmts.is_empty() {
                (None, 0)
            } else {
                (Some(Block::with_stmts(default_stmts)), 0)
            }
        } else {
            (None, 0)
        }
    };

    // If we have no cases, bail
    if cases.is_empty() {
        return None;
    }

    // Sort cases by their first (smallest) value for readable output
    cases.sort_by_key(|c| c.values.iter().copied().min().unwrap_or(u32::MAX));

    let switch = Stmt::Switch {
        index: index_expr,
        cases,
        default: default_block,
    };

    Some((switch, None, consumed))
}

/// Replace Br/BrIf with a specific label to use a new label
fn replace_break_label(stmts: &mut Vec<Stmt>, old_label: u32, new_label: u32) {
    for stmt in stmts.iter_mut() {
        match stmt {
            Stmt::Br { label, is_loop } if *label == old_label && !*is_loop => {
                *label = new_label;
            }
            Stmt::BrIf {
                label, is_loop, ..
            } if *label == old_label && !*is_loop => {
                *label = new_label;
            }
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                replace_break_label(&mut body.stmts, old_label, new_label);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                replace_break_label(&mut then_block.stmts, old_label, new_label);
                if let Some(eb) = else_block {
                    replace_break_label(&mut eb.stmts, old_label, new_label);
                }
            }
            Stmt::Switch {
                cases, default, ..
            } => {
                for case in cases {
                    replace_break_label(&mut case.body.stmts, old_label, new_label);
                }
                if let Some(def) = default {
                    replace_break_label(&mut def.stmts, old_label, new_label);
                }
            }
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                replace_break_label(&mut body.stmts, old_label, new_label);
                replace_break_label(&mut finally_block.stmts, old_label, new_label);
            }
            _ => {}
        }
    }
}

/// Recover if-else from block + if + break pattern.
///
/// Pattern:
/// ```text
/// Block { label: L, body: [
///   If { cond, then_block: [...then..., Br { label: L }], else_block: None },
///   ...else_stmts...
/// ]}
/// ```
/// Becomes: `If { cond, then_block, else_block: Some(else_stmts) }`
fn recover_if_else(block: &mut Block) -> bool {
    let mut changed = false;
    let mut new_stmts = Vec::with_capacity(block.stmts.len());

    for stmt in std::mem::take(&mut block.stmts) {
        if let Stmt::Block {
            label,
            body: mut block_body,
        } = stmt
        {
            if let Some(transformed) = try_transform_to_if_else(label, &mut block_body) {
                new_stmts.push(transformed);
                changed = true;
                continue;
            }
            new_stmts.push(Stmt::Block {
                label,
                body: block_body,
            });
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
    changed
}

fn try_transform_to_if_else(block_label: u32, block_body: &mut Block) -> Option<Stmt> {
    // Block body must start with an If
    if block_body.stmts.is_empty() {
        return None;
    }

    // First stmt must be an If with no else, and then_block ending with Br { label: block_label }
    let is_pattern = match &block_body.stmts[0] {
        Stmt::If {
            then_block,
            else_block: None,
            ..
        } => {
            // then_block must end with Br to block_label
            match then_block.stmts.last() {
                Some(Stmt::Br {
                    label,
                    is_loop: false,
                }) if *label == block_label => true,
                _ => false,
            }
        }
        _ => false,
    };

    if !is_pattern {
        return None;
    }

    // Must have else stmts (stmts after the If)
    if block_body.stmts.len() < 2 {
        return None;
    }

    // Check that else stmts don't reference block_label
    let else_stmts = &block_body.stmts[1..];
    if has_branch_to_label(else_stmts, block_label) {
        return None;
    }

    // Check then_block (minus the trailing Br) doesn't reference block_label
    let if_stmt = block_body.stmts.remove(0);
    let else_stmts: Vec<Stmt> = block_body.stmts.drain(..).collect();

    if let Stmt::If {
        cond,
        mut then_block,
        ..
    } = if_stmt
    {
        // Check then_block body (minus trailing Br) doesn't reference block_label
        let then_without_br = &then_block.stmts[..then_block.stmts.len() - 1];
        if has_branch_to_label(then_without_br, block_label) {
            // Put things back - can't transform
            let mut restored = vec![Stmt::If {
                cond,
                then_block,
                else_block: None,
            }];
            restored.extend(else_stmts);
            block_body.stmts = restored;
            return None;
        }

        // Remove the trailing Br from then_block
        then_block.stmts.pop();

        Some(Stmt::If {
            cond,
            then_block,
            else_block: Some(Block::with_stmts(else_stmts)),
        })
    } else {
        unreachable!()
    }
}

/// Recover block ending with break to itself.
///
/// Pattern: `Block { label: L, body: [...code..., Br { label: L }] }` -> `...code...`
///
/// When a block ends with a Br to its own label, the Br is a no-op since
/// breaking to a block label just jumps to the end of the block.
/// Recover blocks with a single br_if mid-block.
///
/// Pattern:
/// ```text
/// block_N: {
///   code_before;
///   if (cond) break block_N;
///   code_after;
/// }
/// ```
/// Becomes:
/// ```text
/// code_before;
/// if (!cond) {
///   code_after;
/// }
/// ```
fn recover_block_with_single_break(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;

    while i < block.stmts.len() {
        let transformation = check_single_break_pattern(&block.stmts[i]);

        if let Some((before, cond, after)) = transformation {
            block.stmts.remove(i);

            let before_len = before.len();

            // Insert 'before' statements
            for (j, stmt) in before.into_iter().enumerate() {
                block.stmts.insert(i + j, stmt);
            }

            // Insert if statement with negated condition if there's an 'after' part
            if !after.is_empty() {
                let negated = negate_condition(cond);
                block.stmts.insert(
                    i + before_len,
                    Stmt::If {
                        cond: negated,
                        then_block: Block::with_stmts(after),
                        else_block: None,
                    },
                );
            }

            changed = true;
            continue;
        }
        i += 1;
    }

    changed
}

/// Check if a Block statement matches the single br_if pattern:
/// block_N: { before...; br_if N cond; after... }
/// where N has no other branch references.
fn check_single_break_pattern(stmt: &Stmt) -> Option<(Vec<Stmt>, Expr, Vec<Stmt>)> {
    let (label, body) = match stmt {
        Stmt::Block { label, body } => (*label, body),
        _ => return None,
    };

    // Find the first br_if to this label
    let mut br_if_idx = None;
    for (idx, s) in body.stmts.iter().enumerate() {
        if let Stmt::BrIf {
            label: l,
            is_loop: false,
            ..
        } = s
        {
            if *l == label {
                br_if_idx = Some(idx);
                break;
            }
        }
    }

    let idx = br_if_idx?;

    let before = &body.stmts[..idx];
    let after = &body.stmts[idx + 1..];

    // Ensure no other branches (Br, BrIf, BrTable) reference this label
    if has_branch_to_label(before, label) || has_branch_to_label(after, label) {
        return None;
    }

    if let Stmt::BrIf { cond, .. } = &body.stmts[idx] {
        Some((before.to_vec(), cond.clone(), after.to_vec()))
    } else {
        None
    }
}

fn recover_block_ending_with_break(block: &mut Block) -> bool {
    let mut changed = false;
    let mut new_stmts = Vec::with_capacity(block.stmts.len());

    for stmt in std::mem::take(&mut block.stmts) {
        if let Stmt::Block {
            label,
            mut body,
        } = stmt
        {
            // Check if the block ends with Br to its own label
            let ends_with_self_break = match body.stmts.last() {
                Some(Stmt::Br {
                    label: l,
                    is_loop: false,
                }) if *l == label => true,
                _ => false,
            };

            if ends_with_self_break {
                // Remove the trailing self-break
                body.stmts.pop();

                // Check if there are other references to this label
                if !has_branch_to_label(&body.stmts, label) {
                    // No other references - inline the body
                    new_stmts.extend(body.stmts);
                    changed = true;
                } else {
                    // Still has references - keep block but without trailing break
                    new_stmts.push(Stmt::Block { label, body });
                    changed = true;
                }
            } else {
                new_stmts.push(Stmt::Block { label, body });
            }
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
    changed
}

/// Pattern:
/// Loop { body..., BrIf(same_label, cond, is_loop=true) [, break] }
/// → DoWhile { body, cond }
fn recover_do_while(block: &mut Block) -> bool {
    let mut new_stmts = Vec::with_capacity(block.stmts.len());
    let mut changed = false;

    for stmt in std::mem::take(&mut block.stmts) {
        if let Stmt::Loop { label, mut body } = stmt {
            if let Some((do_while, trailing_br)) = try_transform_to_do_while(label, &mut body) {
                new_stmts.push(do_while);
                if let Some(br) = trailing_br {
                    new_stmts.push(br);
                }
                changed = true;
                continue;
            }
            new_stmts.push(Stmt::Loop { label, body });
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
    changed
}

/// Returns (DoWhile stmt, optional trailing Br to emit after the do-while)
fn try_transform_to_do_while(label: u32, body: &mut Block) -> Option<(Stmt, Option<Stmt>)> {
    let len = body.stmts.len();
    if len == 0 {
        return None;
    }

    // Pattern: loop body ends with BrIf(continue) followed optionally by Br(break to somewhere).
    //
    // Case 1: [...body, BrIf { label, is_loop: true }]
    //   → do { body } while (cond);
    //
    // Case 2: [...body, BrIf { label, is_loop: true }, Br { target, is_loop: false }]
    //   → do { body } while (cond); br target;
    //   The trailing Br executes when the condition is false (loop exits).
    let (br_if_idx, has_trailing_br) = if len >= 2 {
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

    // Extract the trailing Br before removing the BrIf
    let trailing_br = if has_trailing_br {
        Some(body.stmts.remove(br_if_idx + 1))
    } else {
        None
    };

    // Extract condition
    let cond = if let Stmt::BrIf { cond, .. } = body.stmts.remove(br_if_idx) {
        cond
    } else {
        unreachable!()
    };

    Some((
        Stmt::DoWhile {
            body: std::mem::take(body),
            cond,
        },
        trailing_br,
    ))
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
fn recover_while(block: &mut Block) -> bool {
    let mut new_stmts = Vec::with_capacity(block.stmts.len());
    let mut changed = false;

    for stmt in std::mem::take(&mut block.stmts) {
        if let Stmt::Block {
            label: block_label,
            body: mut block_body,
        } = stmt
        {
            if let Some(transformed) = try_transform_to_while(block_label, &mut block_body) {
                new_stmts.push(transformed);
                changed = true;
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
    changed
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
            Stmt::Switch {
                cases, default, ..
            } => {
                for case in cases {
                    if has_simple_branch_to_label(&case.body.stmts, label) {
                        return true;
                    }
                }
                if let Some(def) = default {
                    if has_simple_branch_to_label(&def.stmts, label) {
                        return true;
                    }
                }
            }
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                if has_simple_branch_to_label(&body.stmts, label) {
                    return true;
                }
                if has_simple_branch_to_label(&finally_block.stmts, label) {
                    return true;
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
            Stmt::Switch {
                cases, default, ..
            } => {
                for case in cases {
                    if has_br_table_to_label(&case.body.stmts, label) {
                        return true;
                    }
                }
                if let Some(def) = default {
                    if has_br_table_to_label(&def.stmts, label) {
                        return true;
                    }
                }
            }
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                if has_br_table_to_label(&body.stmts, label) {
                    return true;
                }
                if has_br_table_to_label(&finally_block.stmts, label) {
                    return true;
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
fn recover_block_to_early_return(block: &mut Block) -> bool {
    let mut changed = false;
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
                    replace_breaks_with_return(&mut body, label, Some(&ret_val));

                    // Insert block body before LocalSet
                    for (j, stmt) in body.stmts.into_iter().enumerate() {
                        block.stmts.insert(i + j, stmt);
                    }
                    changed = true;
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
                        replace_breaks_with_return(&mut body, label, Some(ret_val));

                        for (j, stmt) in body.stmts.into_iter().enumerate() {
                            block.stmts.insert(i + j, stmt);
                        }
                        changed = true;
                        continue;
                    }
                }
            }
        }

        // Pattern 3: Block + epilog stmts + Return → try/finally
        // Skip if block body has Return stmts — those returns should NOT go
        // through the epilog, but try/finally would force them to.
        // This is critical for asyncify (Go/TinyGo) where returns during
        // normal execution must not run the state-saving epilog.
        if i + 2 < block.stmts.len() {
            if let Stmt::Block { label, body } = &block.stmts[i] {
                let label = *label;
                if !has_br_table_to_label(&body.stmts, label)
                    && !has_return_stmt(&body.stmts)
                {
                    // Find Return in the tail after the block
                    let tail = &block.stmts[i + 1..];
                    let ret_idx = tail.iter().rposition(|s| matches!(s, Stmt::Return(Some(_))));
                    if let Some(ri) = ret_idx {
                        // tail[0..ri] = epilog, tail[ri] = return
                        // Only apply if there's actual epilog (ri > 0) and it's the last stmt
                        if ri > 0 && ri == tail.len() - 1 {
                            let ret_val = if let Stmt::Return(Some(v)) = &tail[ri] {
                                v.clone()
                            } else {
                                unreachable!()
                            };

                            // Check epilog stmts are simple (no control flow)
                            let epilog = &tail[..ri];
                            let epilog_ok = epilog.iter().all(|s| {
                                matches!(
                                    s,
                                    Stmt::LocalSet { .. }
                                        | Stmt::GlobalSet { .. }
                                        | Stmt::Store { .. }
                                        | Stmt::Expr(_)
                                )
                            });

                            if epilog_ok {
                                // Extract and transform
                                let mut body = if let Stmt::Block { body, .. } =
                                    block.stmts.remove(i)
                                {
                                    body
                                } else {
                                    unreachable!()
                                };

                                // Collect epilog and return from remaining stmts
                                let epilog_stmts: Vec<Stmt> =
                                    block.stmts.drain(i..i + ri).collect();
                                let return_stmt = block.stmts.remove(i); // the Return

                                // Replace break L with return in the block body
                                replace_breaks_with_return(&mut body, label, Some(&ret_val));

                                // Add return at end of try body for fallthrough
                                body.stmts.push(return_stmt.clone());

                                let try_finally = Stmt::TryFinally {
                                    body,
                                    finally_block: Block::with_stmts(epilog_stmts),
                                };

                                block.stmts.insert(i, try_finally);
                                changed = true;
                                continue;
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
    changed
}

/// Void function epilog: Block + epilog stmts at end of function body → try/finally
///
/// Only applied at the function body level for void functions (no return value).
/// ```text
/// block_L: { body } epilog_stmts;
/// → try { body_with_breaks_as_void_returns; } finally { epilog_stmts; }
/// ```
/// Void function: if the last stmt is `block_L: { body }` with no epilog,
/// replace all `break L` with `return;` and inline the body.
fn recover_void_block(block: &mut Block) {
    let len = block.stmts.len();
    if len == 0 {
        return;
    }

    // Last stmt must be a Block
    let last_idx = len - 1;
    let (label, has_br_table) = match &block.stmts[last_idx] {
        Stmt::Block { label, body } => (*label, has_br_table_to_label(&body.stmts, *label)),
        _ => return,
    };

    if has_br_table {
        return;
    }

    let mut body = if let Stmt::Block { body, .. } = block.stmts.remove(last_idx) {
        body
    } else {
        unreachable!()
    };

    // Replace break L with void return
    replace_breaks_with_return(&mut body, label, None);

    // Inline body at the position where the block was
    for (j, stmt) in body.stmts.into_iter().enumerate() {
        block.stmts.insert(last_idx + j, stmt);
    }

    // Re-run control flow recovery on the modified body
    recover_control_flow(block);
}

fn recover_void_epilog(block: &mut Block) {
    let len = block.stmts.len();
    if len < 2 {
        return;
    }

    // Find the last Block statement
    let block_idx = block.stmts.iter().rposition(|s| matches!(s, Stmt::Block { .. }));
    let block_idx = match block_idx {
        Some(idx) if idx + 1 < len => idx, // must have epilog after it
        _ => return,
    };

    let (label, has_br_table) = if let Stmt::Block { label, body } = &block.stmts[block_idx] {
        (*label, has_br_table_to_label(&body.stmts, *label))
    } else {
        return;
    };

    if has_br_table {
        return;
    }

    // Don't use try/finally if block body has Return statements.
    // Returns inside the block skip the epilog in the original code,
    // but try/finally would force them through finally (the epilog).
    // This is critical for asyncify (Go/TinyGo) where returns during
    // normal execution must NOT run the asyncify state-saving epilog.
    if let Stmt::Block { body, .. } = &block.stmts[block_idx] {
        if has_return_stmt(&body.stmts) {
            return;
        }
    }

    // Tail = stmts after the block (must be epilog-only, no Return)
    let tail = &block.stmts[block_idx + 1..];
    if tail.is_empty() || tail.iter().any(|s| matches!(s, Stmt::Return(_))) {
        return;
    }

    let epilog_ok = tail.iter().all(|s| {
        matches!(
            s,
            Stmt::LocalSet { .. } | Stmt::GlobalSet { .. } | Stmt::Store { .. } | Stmt::Expr(_)
        )
    });

    if !epilog_ok {
        return;
    }

    let mut body = if let Stmt::Block { body, .. } = block.stmts.remove(block_idx) {
        body
    } else {
        unreachable!()
    };

    let epilog_stmts: Vec<Stmt> = block.stmts.drain(block_idx..).collect();

    // Replace break L with void return
    replace_breaks_with_return(&mut body, label, None);

    let try_finally = Stmt::TryFinally {
        body,
        finally_block: Block::with_stmts(epilog_stmts),
    };

    block.stmts.insert(block_idx, try_finally);

    // Run control flow recovery on the try body to simplify inner blocks.
    // Also run recover_block_with_terminal_body — it's safe here because
    // returns inside the try body go through finally (the epilog).
    if let Stmt::TryFinally { body, .. } = &mut block.stmts[block_idx] {
        recover_control_flow(body);
        recover_block_with_terminal_body(body);
    }
}

/// Recover blocks whose body ends with a terminator (Return), by inlining
/// the tail (stmts after the block) at each break site.
///
/// Pattern:
/// ```text
/// block_L: {
///   ...body...
///   return X;
/// }
/// tail_stmt_1;
/// tail_stmt_2;
/// ```
/// Since tail is only reachable via `break L`, replace each break with
/// `{ tail_stmts; return; }` and inline the block body.
fn recover_block_with_terminal_body(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < block.stmts.len() {
        if let Stmt::Block { label, body } = &block.stmts[i] {
            let label = *label;
            let body_terminates = matches!(body.stmts.last(), Some(Stmt::Return(_)));
            if body_terminates && !has_br_table_to_label(&body.stmts, label) {
                let tail = &block.stmts[i + 1..];
                if !tail.is_empty() && !has_branch_to_label(tail, label) {
                    // Build the inline replacement: tail + return
                    let mut inline_tail: Vec<Stmt> = tail.to_vec();
                    if !matches!(inline_tail.last(), Some(Stmt::Return(_))) {
                        // Only safe to add void return when body also returns void
                        let is_void = matches!(body.stmts.last(), Some(Stmt::Return(None)));
                        if !is_void {
                            i += 1;
                            continue;
                        }
                        inline_tail.push(Stmt::Return(None));
                    }

                    // Extract block body
                    let mut body =
                        if let Stmt::Block { body, .. } = block.stmts.remove(i) {
                            body
                        } else {
                            unreachable!()
                        };

                    // Remove original tail stmts
                    block.stmts.drain(i..);

                    // Replace breaks with inlined tail
                    replace_breaks_with_inline_tail(&mut body, label, &inline_tail);

                    // Inline the block body
                    for (j, stmt) in body.stmts.into_iter().enumerate() {
                        block.stmts.insert(i + j, stmt);
                    }

                    changed = true;
                    continue;
                }
            }
        }
        i += 1;
    }
    changed
}

/// Replace `Br { label }` and `BrIf { label, cond }` with inline tail statements
fn replace_breaks_with_inline_tail(block: &mut Block, target_label: u32, tail: &[Stmt]) {
    let stmts = std::mem::take(&mut block.stmts);
    let mut new_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts {
        match stmt {
            Stmt::Br {
                label,
                is_loop: false,
            } if label == target_label => {
                new_stmts.extend(tail.iter().cloned());
            }
            Stmt::BrIf {
                label,
                cond,
                is_loop: false,
            } if label == target_label => {
                new_stmts.push(Stmt::If {
                    cond,
                    then_block: Block::with_stmts(tail.to_vec()),
                    else_block: None,
                });
            }
            // Recurse into nested structures
            Stmt::Block { label, mut body } => {
                replace_breaks_with_inline_tail(&mut body, target_label, tail);
                new_stmts.push(Stmt::Block { label, body });
            }
            Stmt::Loop { label, mut body } => {
                replace_breaks_with_inline_tail(&mut body, target_label, tail);
                new_stmts.push(Stmt::Loop { label, body });
            }
            Stmt::If {
                cond,
                mut then_block,
                mut else_block,
            } => {
                replace_breaks_with_inline_tail(&mut then_block, target_label, tail);
                if let Some(ref mut eb) = else_block {
                    replace_breaks_with_inline_tail(eb, target_label, tail);
                }
                new_stmts.push(Stmt::If {
                    cond,
                    then_block,
                    else_block,
                });
            }
            Stmt::DoWhile { mut body, cond } => {
                replace_breaks_with_inline_tail(&mut body, target_label, tail);
                new_stmts.push(Stmt::DoWhile { body, cond });
            }
            Stmt::While { cond, mut body } => {
                replace_breaks_with_inline_tail(&mut body, target_label, tail);
                new_stmts.push(Stmt::While { cond, body });
            }
            Stmt::Switch {
                index,
                mut cases,
                mut default,
            } => {
                for case in &mut cases {
                    replace_breaks_with_inline_tail(&mut case.body, target_label, tail);
                }
                if let Some(ref mut def) = default {
                    replace_breaks_with_inline_tail(def, target_label, tail);
                }
                new_stmts.push(Stmt::Switch {
                    index,
                    cases,
                    default,
                });
            }
            Stmt::TryFinally {
                mut body,
                mut finally_block,
            } => {
                replace_breaks_with_inline_tail(&mut body, target_label, tail);
                replace_breaks_with_inline_tail(&mut finally_block, target_label, tail);
                new_stmts.push(Stmt::TryFinally {
                    body,
                    finally_block,
                });
            }
            other => new_stmts.push(other),
        }
    }

    block.stmts = new_stmts;
}

/// Replace `Br { label }` and `BrIf { label, cond }` with return statements
fn replace_breaks_with_return(block: &mut Block, target_label: u32, ret_val: Option<&Expr>) {
    let stmts = std::mem::take(&mut block.stmts);
    let mut new_stmts = Vec::with_capacity(stmts.len());

    for stmt in stmts {
        match stmt {
            // br label -> return X
            Stmt::Br {
                label,
                is_loop: false,
            } if label == target_label => {
                new_stmts.push(Stmt::Return(ret_val.cloned()));
            }
            // br_if label, cond -> if (cond) return X
            Stmt::BrIf {
                label,
                cond,
                is_loop: false,
            } if label == target_label => {
                new_stmts.push(Stmt::If {
                    cond,
                    then_block: Block::with_stmts(vec![Stmt::Return(ret_val.cloned())]),
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
            Stmt::Switch {
                index,
                mut cases,
                mut default,
            } => {
                for case in &mut cases {
                    replace_breaks_with_return(&mut case.body, target_label, ret_val);
                }
                if let Some(ref mut def) = default {
                    replace_breaks_with_return(def, target_label, ret_val);
                }
                new_stmts.push(Stmt::Switch {
                    index,
                    cases,
                    default,
                });
            }
            Stmt::TryFinally {
                mut body,
                mut finally_block,
            } => {
                replace_breaks_with_return(&mut body, target_label, ret_val);
                replace_breaks_with_return(&mut finally_block, target_label, ret_val);
                new_stmts.push(Stmt::TryFinally {
                    body,
                    finally_block,
                });
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
fn recover_block_to_unreachable(block: &mut Block) -> bool {
    let mut changed = false;
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
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
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
            Stmt::Switch {
                index,
                mut cases,
                mut default,
            } => {
                for case in &mut cases {
                    replace_breaks_with_unreachable(&mut case.body, target_label);
                }
                if let Some(ref mut def) = default {
                    replace_breaks_with_unreachable(def, target_label);
                }
                new_stmts.push(Stmt::Switch {
                    index,
                    cases,
                    default,
                });
            }
            Stmt::TryFinally {
                mut body,
                mut finally_block,
            } => {
                replace_breaks_with_unreachable(&mut body, target_label);
                replace_breaks_with_unreachable(&mut finally_block, target_label);
                new_stmts.push(Stmt::TryFinally {
                    body,
                    finally_block,
                });
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
fn remove_unused_block_wrappers(block: &mut Block) -> bool {
    // First, collect all labels that are referenced by BrTable in this block
    // (since BrTable can reference sibling blocks)
    let mut br_table_labels = HashSet::new();
    collect_br_table_labels(&block.stmts, &mut br_table_labels);

    let stmts = std::mem::take(&mut block.stmts);
    let mut new_stmts = Vec::with_capacity(stmts.len());
    let mut changed = false;

    for stmt in stmts {
        if let Stmt::Block { label, body } = stmt {
            // Check if any statement in the body references this label
            // OR if any BrTable in the parent block references it
            if !has_branch_to_label(&body.stmts, label) && !br_table_labels.contains(&label) {
                // No breaks to this label - inline the body
                new_stmts.extend(body.stmts);
                changed = true;
            } else {
                // Keep the block
                new_stmts.push(Stmt::Block { label, body });
            }
        } else {
            new_stmts.push(stmt);
        }
    }

    block.stmts = new_stmts;
    changed
}

/// Collect all labels referenced by BrTable statements
fn collect_br_table_labels(stmts: &[Stmt], labels: &mut HashSet<u32>) {
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
            Stmt::Switch {
                cases, default, ..
            } => {
                for case in cases {
                    collect_br_table_labels(&case.body.stmts, labels);
                }
                if let Some(def) = default {
                    collect_br_table_labels(&def.stmts, labels);
                }
            }
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                collect_br_table_labels(&body.stmts, labels);
                collect_br_table_labels(&finally_block.stmts, labels);
            }
            _ => {}
        }
    }
}

/// Check if any statement contains a Return (recursively)
fn has_return_stmt(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Return(_) => return true,
            Stmt::Block { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::DoWhile { body, .. }
            | Stmt::While { body, .. } => {
                if has_return_stmt(&body.stmts) {
                    return true;
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if has_return_stmt(&then_block.stmts) {
                    return true;
                }
                if let Some(eb) = else_block {
                    if has_return_stmt(&eb.stmts) {
                        return true;
                    }
                }
            }
            Stmt::Switch {
                cases, default, ..
            } => {
                for case in cases {
                    if has_return_stmt(&case.body.stmts) {
                        return true;
                    }
                }
                if let Some(def) = default {
                    if has_return_stmt(&def.stmts) {
                        return true;
                    }
                }
            }
            Stmt::TryFinally {
                body,
                finally_block,
            } => {
                if has_return_stmt(&body.stmts) || has_return_stmt(&finally_block.stmts) {
                    return true;
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
