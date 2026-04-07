//! Direct IR transformation for stackifier
//!
//! Transforms block/br patterns to structured code without building a CFG.

use crate::ir::{Block, CmpOp, Expr, ExprKind, InferredType, Stmt, UnaryOp};
use crate::passes::PassContext;

/// Transform a function body
pub fn transform_function(body: &mut Block, _ctx: &mut PassContext) {
    // Apply transformations iteratively until no changes
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: u32 = 100;

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        // First recurse into nested structures
        recurse_into_nested(body);

        // Then apply transformations at this level
        changed |= transform_block_to_if(body);
        changed |= transform_block_with_single_break(body);
        changed |= transform_loop_with_break_at_start(body);
        changed |= transform_block_ending_with_break(body);
        changed |= transform_if_break_pattern(body);
        changed |= transform_nested_block_fallthrough(body);
        changed |= inline_trivial_blocks(body);
    }
}

/// Recurse into nested control structures
fn recurse_into_nested(block: &mut Block) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Block { body, .. } | Stmt::Loop { body, .. } => {
                transform_function(body, &mut PassContext::new());
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                transform_function(then_block, &mut PassContext::new());
                if let Some(eb) = else_block {
                    transform_function(eb, &mut PassContext::new());
                }
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                transform_function(body, &mut PassContext::new());
            }
            _ => {}
        }
    }
}

/// Transform: block_N: { if (cond) break block_N; ...rest... }
/// Into: if (!cond) { ...rest... }
fn transform_block_to_if(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;

    while i < block.stmts.len() {
        let should_transform = if let Stmt::Block { label, body } = &block.stmts[i] {
            // Check if first statement is br_if to this block's label
            if body.stmts.is_empty() {
                false
            } else if let Stmt::BrIf {
                label: br_label,
                is_loop: false,
                ..
            } = &body.stmts[0]
            {
                *br_label == *label
                    && !has_other_breaks_to_label(&body.stmts[1..], *label)
                    && !has_br_table_to_label(&body.stmts, *label)
            } else {
                false
            }
        } else {
            false
        };

        if should_transform {
            let stmt = block.stmts.remove(i);
            if let Stmt::Block { body, .. } = stmt {
                let mut body_stmts = body.stmts;
                // Extract condition from first br_if
                if let Stmt::BrIf { cond, .. } = body_stmts.remove(0) {
                    // Negate condition and wrap rest in if
                    let negated = negate_condition(cond);
                    let then_block = Block::with_stmts(body_stmts);

                    // Only add if there's content
                    if !then_block.stmts.is_empty() {
                        block.stmts.insert(
                            i,
                            Stmt::If {
                                cond: negated,
                                then_block,
                                else_block: None,
                            },
                        );
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

/// Transform: block_N: { ...code...; br_if block_N cond; ...more... }
/// Where br_if is followed by code and then implicit fallthrough
/// Into: ...code...; if (cond) { } else { ...more... }
fn transform_block_with_single_break(block: &mut Block) -> bool {
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

            // Insert if statement if there's an 'after' part
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

/// Check if a block matches the single break pattern
fn check_single_break_pattern(stmt: &Stmt) -> Option<(Vec<Stmt>, Expr, Vec<Stmt>)> {
    if let Stmt::Block { label, body } = stmt {
        // Find br_if to this label
        let mut br_if_idx = None;
        for (idx, s) in body.stmts.iter().enumerate() {
            if let Stmt::BrIf {
                label: l,
                is_loop: false,
                ..
            } = s
            {
                if *l == *label {
                    br_if_idx = Some(idx);
                    break;
                }
            }
        }

        if let Some(idx) = br_if_idx {
            // Check no other breaks to this label
            let before = &body.stmts[..idx];
            let after = &body.stmts[idx + 1..];

            if !has_other_breaks_to_label(before, *label)
                && !has_other_breaks_to_label(after, *label)
                && !has_br_table_to_label(&body.stmts, *label)
            {
                if let Stmt::BrIf { cond, .. } = &body.stmts[idx] {
                    return Some((before.to_vec(), cond.clone(), after.to_vec()));
                }
            }
        }
    }
    None
}

/// Transform: loop_N: { br_if block_M cond; ...body...; br loop_N }
/// (Where block_M is the outer block)
/// Into: while (!cond) { ...body... }
fn transform_loop_with_break_at_start(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;

    while i < block.stmts.len() {
        // Look for Block { Loop { br_if Block; ...; br Loop } }
        let transformation = check_while_pattern(&block.stmts[i]);

        if let Some((cond, body_stmts)) = transformation {
            block.stmts.remove(i);
            block.stmts.insert(
                i,
                Stmt::While {
                    cond,
                    body: Block::with_stmts(body_stmts),
                },
            );
            changed = true;
            continue;
        }
        i += 1;
    }

    changed
}

/// Check for while loop pattern: Block { Loop { br_if Block; ...; br Loop } }
fn check_while_pattern(stmt: &Stmt) -> Option<(Expr, Vec<Stmt>)> {
    if let Stmt::Block {
        label: block_label,
        body: block_body,
    } = stmt
    {
        if block_body.stmts.len() == 1 {
            if let Stmt::Loop {
                label: loop_label,
                body: loop_body,
            } = &block_body.stmts[0]
            {
                if loop_body.stmts.len() >= 2 {
                    // First: br_if to block (exit condition)
                    let exit_cond = if let Stmt::BrIf {
                        label,
                        cond,
                        is_loop: false,
                    } = &loop_body.stmts[0]
                    {
                        if *label == *block_label {
                            Some(cond.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Last: br to loop (continue)
                    let has_continue = if let Stmt::Br {
                        label,
                        is_loop: true,
                    } = &loop_body.stmts[loop_body.stmts.len() - 1]
                    {
                        *label == *loop_label
                    } else {
                        false
                    };

                    if let Some(cond) = exit_cond {
                        if has_continue {
                            // Check no other breaks to block or loop
                            let middle = &loop_body.stmts[1..loop_body.stmts.len() - 1];
                            if !has_branch_to_label(middle, *block_label)
                                && !has_branch_to_label(middle, *loop_label)
                            {
                                let body_stmts = middle.to_vec();
                                let while_cond = negate_condition(cond);
                                return Some((while_cond, body_stmts));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Inline blocks that have no breaks to their label
fn inline_trivial_blocks(block: &mut Block) -> bool {
    // Collect labels referenced by BrTable in this block (sibling references)
    let br_table_labels = collect_br_table_labels_set(&block.stmts);

    let mut changed = false;
    let mut i = 0;

    while i < block.stmts.len() {
        let should_inline = if let Stmt::Block { label, body } = &block.stmts[i] {
            // Don't inline if BrTable references this label from outside
            !has_branch_to_label(&body.stmts, *label) && !br_table_labels.contains(label)
        } else {
            false
        };

        if should_inline {
            let stmt = block.stmts.remove(i);
            if let Stmt::Block { body, .. } = stmt {
                // Insert all body statements at current position
                for (j, s) in body.stmts.into_iter().enumerate() {
                    block.stmts.insert(i + j, s);
                }
                changed = true;
                continue;
            }
        }
        i += 1;
    }

    changed
}

/// Collect all labels referenced by BrTable statements into a HashSet
fn collect_br_table_labels_set(stmts: &[Stmt]) -> std::collections::HashSet<u32> {
    let mut labels = std::collections::HashSet::new();
    collect_br_table_labels_recursive(stmts, &mut labels);
    labels
}

fn collect_br_table_labels_recursive(stmts: &[Stmt], labels: &mut std::collections::HashSet<u32>) {
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
                collect_br_table_labels_recursive(&body.stmts, labels);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_br_table_labels_recursive(&then_block.stmts, labels);
                if let Some(eb) = else_block {
                    collect_br_table_labels_recursive(&eb.stmts, labels);
                }
            }
            _ => {}
        }
    }
}

/// Transform: block_N: { ...code...; break block_N; }
/// Where the block ends with unconditional break to itself
/// Into: ...code...
fn transform_block_ending_with_break(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;

    while i < block.stmts.len() {
        let should_transform = if let Stmt::Block { label, body } = &block.stmts[i] {
            if body.stmts.is_empty() {
                false
            } else {
                // Check if last statement is br to this label
                let last = &body.stmts[body.stmts.len() - 1];
                if let Stmt::Br {
                    label: br_label,
                    is_loop: false,
                } = last
                {
                    *br_label == *label
                        && !has_other_breaks_to_label(&body.stmts[..body.stmts.len() - 1], *label)
                        && !has_br_table_to_label(&body.stmts, *label)
                } else {
                    false
                }
            }
        } else {
            false
        };

        if should_transform {
            let stmt = block.stmts.remove(i);
            if let Stmt::Block { mut body, .. } = stmt {
                // Remove the trailing break
                body.stmts.pop();
                // Insert remaining statements
                for (j, s) in body.stmts.into_iter().enumerate() {
                    block.stmts.insert(i + j, s);
                }
                changed = true;
                continue;
            }
        }
        i += 1;
    }

    changed
}

/// Transform: block_N: { if (cond) { ...then...; break block_N; } ...else... }
/// Into: if (cond) { ...then... } else { ...else... }
fn transform_if_break_pattern(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;

    while i < block.stmts.len() {
        let transformation = check_if_break_pattern(&block.stmts[i]);

        if let Some((cond, then_stmts, else_stmts)) = transformation {
            block.stmts.remove(i);

            // Build if-else
            let then_block = Block::with_stmts(then_stmts);
            let else_block = if else_stmts.is_empty() {
                None
            } else {
                Some(Block::with_stmts(else_stmts))
            };

            block.stmts.insert(
                i,
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                },
            );

            changed = true;
            continue;
        }
        i += 1;
    }

    changed
}

/// Check for: block_N: { if (cond) { ...then...; break block_N; } ...else... }
fn check_if_break_pattern(stmt: &Stmt) -> Option<(Expr, Vec<Stmt>, Vec<Stmt>)> {
    if let Stmt::Block { label, body } = stmt {
        if body.stmts.is_empty() {
            return None;
        }

        // First statement must be an if
        if let Stmt::If {
            cond,
            then_block,
            else_block: None, // Must not have else already
        } = &body.stmts[0]
        {
            // Then block must end with break to this label
            if then_block.stmts.is_empty() {
                return None;
            }

            let then_last = &then_block.stmts[then_block.stmts.len() - 1];
            if let Stmt::Br {
                label: br_label,
                is_loop: false,
            } = then_last
            {
                if *br_label == *label {
                    // Check no other breaks in then block
                    let then_without_break = &then_block.stmts[..then_block.stmts.len() - 1];
                    if has_branch_to_label(then_without_break, *label) {
                        return None;
                    }

                    // Get else statements (everything after the if in the block)
                    let else_stmts: Vec<_> = body.stmts[1..].to_vec();

                    // Check else statements end with break or have no complex breaks
                    let else_clean = if else_stmts.is_empty() {
                        true
                    } else {
                        // Check if last is break to this label
                        let else_last = &else_stmts[else_stmts.len() - 1];
                        if let Stmt::Br {
                            label: br_label,
                            is_loop: false,
                        } = else_last
                        {
                            *br_label == *label
                                && !has_other_breaks_to_label(
                                    &else_stmts[..else_stmts.len() - 1],
                                    *label,
                                )
                        } else {
                            // Else falls through - that's ok too
                            !has_branch_to_label(&else_stmts, *label)
                        }
                    };

                    if else_clean && !has_br_table_to_label(&body.stmts, *label) {
                        let then_clean: Vec<_> = then_without_break.to_vec();
                        let mut else_clean: Vec<_> = else_stmts;

                        // Remove trailing break from else if present
                        if !else_clean.is_empty() {
                            if let Stmt::Br { is_loop: false, .. } = else_clean.last().unwrap() {
                                else_clean.pop();
                            }
                        }

                        return Some((cond.clone(), then_clean, else_clean));
                    }
                }
            }
        }
    }
    None
}

/// Transform nested blocks where inner ends with break to outer
/// block_outer: { ...code...; block_inner: { ...inner...; break block_outer; } }
/// Into: block_outer: { ...code...; ...inner... }
fn transform_nested_block_fallthrough(block: &mut Block) -> bool {
    let mut changed = false;

    // Process each block statement
    for stmt in &mut block.stmts {
        if let Stmt::Block {
            label: outer_label,
            body: outer_body,
        } = stmt
        {
            // Look for inner blocks that end with break to outer
            let mut i = 0;
            while i < outer_body.stmts.len() {
                let should_inline = if let Stmt::Block {
                    label: inner_label,
                    body: inner_body,
                } = &outer_body.stmts[i]
                {
                    // Check if inner block ends with break to outer
                    if let Some(Stmt::Br {
                        label,
                        is_loop: false,
                    }) = inner_body.stmts.last()
                    {
                        *label == *outer_label
                            && !has_other_breaks_to_label(
                                &inner_body.stmts[..inner_body.stmts.len() - 1],
                                *inner_label,
                            )
                            && !has_br_table_to_label(&inner_body.stmts, *inner_label)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_inline {
                    let inner_stmt = outer_body.stmts.remove(i);
                    if let Stmt::Block {
                        body: mut inner_body,
                        ..
                    } = inner_stmt
                    {
                        // Remove the trailing break
                        inner_body.stmts.pop();
                        // Insert inner body statements
                        for (j, s) in inner_body.stmts.into_iter().enumerate() {
                            outer_body.stmts.insert(i + j, s);
                        }
                        changed = true;
                        continue;
                    }
                }
                i += 1;
            }
        }
    }

    changed
}

/// Check if stmts contain Br or BrIf to label (excluding first check)
fn has_other_breaks_to_label(stmts: &[Stmt], label: u32) -> bool {
    for stmt in stmts {
        if has_branch_to_label_in_stmt(stmt, label) {
            return true;
        }
    }
    false
}

/// Check if a statement or its children reference the label
fn has_branch_to_label_in_stmt(stmt: &Stmt, label: u32) -> bool {
    match stmt {
        Stmt::Br { label: l, .. } | Stmt::BrIf { label: l, .. } if *l == label => true,
        Stmt::Block { body, .. } | Stmt::Loop { body, .. } => {
            has_branch_to_label(&body.stmts, label)
        }
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            has_branch_to_label(&then_block.stmts, label)
                || else_block
                    .as_ref()
                    .map(|b| has_branch_to_label(&b.stmts, label))
                    .unwrap_or(false)
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            has_branch_to_label(&body.stmts, label)
        }
        _ => false,
    }
}

/// Check if any statement references the label
fn has_branch_to_label(stmts: &[Stmt], label: u32) -> bool {
    stmts.iter().any(|s| has_branch_to_label_in_stmt(s, label))
}

/// Check if any BrTable references the label
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
            | Stmt::While { body, .. }
            | Stmt::DoWhile { body, .. } => {
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

/// Negate a condition expression
fn negate_condition(cond: Expr) -> Expr {
    match cond.kind {
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
        ExprKind::UnaryOp(UnaryOp::Eqz, inner) => *inner,
        _ => Expr::with_type(
            ExprKind::UnaryOp(UnaryOp::Eqz, Box::new(cond)),
            InferredType::Bool,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negate_eq() {
        let cond = Expr::new(ExprKind::Compare(
            CmpOp::Eq,
            Box::new(Expr::i32_const(1)),
            Box::new(Expr::i32_const(2)),
            InferredType::I32,
        ));
        let negated = negate_condition(cond);
        assert!(matches!(
            negated.kind,
            ExprKind::Compare(CmpOp::Ne, _, _, _)
        ));
    }

    #[test]
    fn test_negate_eqz() {
        let inner = Expr::i32_const(42);
        let cond = Expr::new(ExprKind::UnaryOp(UnaryOp::Eqz, Box::new(inner.clone())));
        let negated = negate_condition(cond);
        assert!(matches!(negated.kind, ExprKind::I32Const(42)));
    }
}
