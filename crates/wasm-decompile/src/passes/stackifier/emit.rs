//! CFG-based structured code emitter
//!
//! Emits structured code (while, if-else) from CFG analysis.

use std::collections::{HashMap, HashSet};

use crate::cfg::{
    build::build_cfg, dominators::compute_dominators, loops::analyze_loops, Cfg, NodeId,
    Terminator,
};
use crate::ir::{Block, CmpOp, Expr, ExprKind, InferredType, Stmt, UnaryOp};

/// Emit structured code from a function body using CFG analysis
pub fn emit_structured(body: &Block) -> Block {
    let cfg = build_cfg(body);
    if cfg.is_empty() {
        return body.clone();
    }

    let dom = compute_dominators(&cfg);
    let loop_info = analyze_loops(&cfg, &dom);

    // Build loop info map
    let mut loop_headers: HashSet<NodeId> = HashSet::new();
    let mut loop_bodies: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
    let mut back_edges: HashSet<(NodeId, NodeId)> = HashSet::new();

    for lp in &loop_info.loops {
        loop_headers.insert(lp.header);
        loop_bodies.insert(lp.header, lp.body.clone());
        for &(src, dst) in &lp.back_edges {
            back_edges.insert((src, dst));
        }
    }

    let mut ctx = EmitCtx {
        cfg: &cfg,
        loop_headers,
        loop_bodies,
        back_edges,
        emitted: HashSet::new(),
    };

    let mut stmts = emit_from(&mut ctx, cfg.entry, None);

    // Check if the emitted code ends with a return at the top level
    let ends_with_return = stmts.last().map(|s| matches!(s, Stmt::Return(_))).unwrap_or(false);

    // If not, find a return value and add it
    // (This happens when all returns are inside if branches)
    if !ends_with_return {
        // Find a return node to get the return expression
        for node_id in 0..cfg.nodes.len() {
            if let Some(node) = cfg.get_node(node_id) {
                if let Terminator::Return(ref val) = node.terminator {
                    // Add the return at the end
                    stmts.extend(node.stmts.clone());
                    stmts.push(Stmt::Return(val.clone()));
                    break;
                }
            }
        }
    }

    Block::with_stmts(stmts)
}

struct EmitCtx<'a> {
    cfg: &'a Cfg,
    loop_headers: HashSet<NodeId>,
    loop_bodies: HashMap<NodeId, HashSet<NodeId>>,
    back_edges: HashSet<(NodeId, NodeId)>,
    emitted: HashSet<NodeId>,
}

impl EmitCtx<'_> {
    fn is_loop_header(&self, n: NodeId) -> bool {
        self.loop_headers.contains(&n)
    }

    fn is_back_edge(&self, from: NodeId, to: NodeId) -> bool {
        self.back_edges.contains(&(from, to))
    }

    fn loop_body(&self, header: NodeId) -> Option<&HashSet<NodeId>> {
        self.loop_bodies.get(&header)
    }

    fn is_in_loop(&self, header: NodeId, node: NodeId) -> bool {
        self.loop_bodies
            .get(&header)
            .map(|b| b.contains(&node))
            .unwrap_or(false)
    }
}

/// Context for emit_from - tracks current loop for break handling
struct EmitRegionCtx {
    /// Stop at this node
    stop_at: Option<NodeId>,
    /// If inside a loop, the loop body (for detecting exits that need break)
    loop_body: Option<HashSet<NodeId>>,
}

impl EmitRegionCtx {
    fn new(stop_at: Option<NodeId>) -> Self {
        Self { stop_at, loop_body: None }
    }

    fn with_loop(stop_at: Option<NodeId>, loop_body: HashSet<NodeId>) -> Self {
        Self { stop_at, loop_body: Some(loop_body) }
    }

    fn is_loop_exit(&self, target: NodeId) -> bool {
        if let Some(ref body) = self.loop_body {
            !body.contains(&target)
        } else {
            false
        }
    }
}

/// Emit code from a starting node until we hit stop condition
fn emit_from(ctx: &mut EmitCtx, start: NodeId, stop_at: Option<NodeId>) -> Vec<Stmt> {
    emit_from_inner(ctx, start, &EmitRegionCtx::new(stop_at))
}

/// Emit code from inside a loop body
fn emit_from_in_loop(ctx: &mut EmitCtx, start: NodeId, stop_at: Option<NodeId>, loop_body: HashSet<NodeId>) -> Vec<Stmt> {
    emit_from_inner(ctx, start, &EmitRegionCtx::with_loop(stop_at, loop_body))
}

fn emit_from_inner(ctx: &mut EmitCtx, start: NodeId, region: &EmitRegionCtx) -> Vec<Stmt> {
    let mut result = Vec::new();
    let mut current = start;

    loop {
        // Stop conditions
        if Some(current) == region.stop_at && current != start {
            break;
        }
        if ctx.emitted.contains(&current) {
            break;
        }
        // If we're in a loop and this node is outside the loop body, it's an exit
        if region.is_loop_exit(current) {
            // Don't process exit nodes - they'll be handled after the loop
            break;
        }

        // Loop header - emit as while/do-while
        if ctx.is_loop_header(current) {
            let loop_stmts = emit_loop(ctx, current);
            result.extend(loop_stmts);

            // Continue after loop exit
            if let Some(exit) = find_loop_exit(ctx, current) {
                if !ctx.emitted.contains(&exit) && Some(exit) != region.stop_at {
                    current = exit;
                    continue;
                }
            }
            break;
        }

        // Mark emitted
        ctx.emitted.insert(current);

        let node = match ctx.cfg.get_node(current) {
            Some(n) => n,
            None => break,
        };

        // Emit node statements
        result.extend(node.stmts.clone());

        // Handle terminator
        match &node.terminator {
            Terminator::Fallthrough(next) | Terminator::Goto(next) => {
                // Back edge = end of loop iteration, stop here
                if ctx.is_back_edge(current, *next) {
                    break;
                }
                // If going to loop exit, stop (implicit break)
                if region.is_loop_exit(*next) {
                    break;
                }
                current = *next;
            }

            Terminator::Branch { cond, then_target, else_target } => {
                let then_is_exit = region.is_loop_exit(*then_target);
                let else_is_exit = region.is_loop_exit(*else_target);

                match (then_is_exit, else_is_exit) {
                    // then exits loop, else stays in loop
                    (true, false) => {
                        // Emit: if (cond) break; else_body
                        result.push(Stmt::If {
                            cond: cond.clone(),
                            then_block: Block::with_stmts(vec![Stmt::Br { label: u32::MAX, is_loop: false }]),
                            else_block: None,
                        });
                        // Continue with else path
                        if !ctx.emitted.contains(else_target) {
                            current = *else_target;
                            continue;
                        }
                        break;
                    }
                    // else exits loop, then stays in loop
                    (false, true) => {
                        // Emit: if (!cond) break; then_body
                        result.push(Stmt::If {
                            cond: negate(cond.clone()),
                            then_block: Block::with_stmts(vec![Stmt::Br { label: u32::MAX, is_loop: false }]),
                            else_block: None,
                        });
                        // Continue with then path
                        if !ctx.emitted.contains(then_target) {
                            current = *then_target;
                            continue;
                        }
                        break;
                    }
                    // Both exit or both stay - handle normally
                    _ => {
                        // Emit if-else
                        let merge = find_merge(ctx.cfg, *then_target, *else_target, region.stop_at);

                        let then_stmts = emit_from_inner(ctx, *then_target, region);
                        let else_stmts = emit_from_inner(ctx, *else_target, region);

                        result.push(Stmt::If {
                            cond: cond.clone(),
                            then_block: Block::with_stmts(then_stmts),
                            else_block: if else_stmts.is_empty() {
                                None
                            } else {
                                Some(Block::with_stmts(else_stmts))
                            },
                        });

                        // Continue from merge
                        if let Some(m) = merge {
                            if !ctx.emitted.contains(&m) && Some(m) != region.stop_at && !region.is_loop_exit(m) {
                                current = m;
                                continue;
                            }
                        }
                        break;
                    }
                }
            }

            Terminator::Switch { index, targets, default } => {
                // Emit as if-else chain
                for (i, &target) in targets.iter().enumerate() {
                    if !ctx.emitted.contains(&target) {
                        let target_stmts = emit_from_inner(ctx, target, region);
                        result.push(Stmt::If {
                            cond: Expr::new(ExprKind::Compare(
                                CmpOp::Eq,
                                Box::new(index.clone()),
                                Box::new(Expr::i32_const(i as i32)),
                            )),
                            then_block: Block::with_stmts(target_stmts),
                            else_block: None,
                        });
                    }
                }
                if !ctx.emitted.contains(default) {
                    let default_stmts = emit_from_inner(ctx, *default, region);
                    result.extend(default_stmts);
                }
                break;
            }

            Terminator::Return(val) => {
                result.push(Stmt::Return(val.clone()));
                break;
            }

            Terminator::Unreachable => break,
        }
    }

    result
}

/// Emit a loop structure
fn emit_loop(ctx: &mut EmitCtx, header: NodeId) -> Vec<Stmt> {
    ctx.emitted.insert(header);

    let node = match ctx.cfg.get_node(header) {
        Some(n) => n,
        None => return Vec::new(),
    };

    // Collect any statements at the header
    let header_stmts = node.stmts.clone();

    let loop_body = ctx.loop_body(header).cloned().unwrap_or_default();

    match &node.terminator {
        Terminator::Branch { cond, then_target, else_target } => {
            // Check if branch targets are the header itself (immediate continue)
            let then_is_header = *then_target == header;
            let else_is_header = *else_target == header;

            // Check if targets are in loop body vs exit
            let in_loop_then = ctx.is_in_loop(header, *then_target);
            let in_loop_else = ctx.is_in_loop(header, *else_target);

            // Pattern: if (cond) continue; body...
            // then_target = header (continue), else_target = body
            if then_is_header && in_loop_else && !else_is_header {
                let body = emit_from_in_loop(ctx, *else_target, Some(header), loop_body.clone());

                // If header has no statements, we can simplify to while(!cond) { body }
                if header_stmts.is_empty() {
                    return vec![Stmt::While {
                        cond: negate(cond.clone()),
                        body: Block::with_stmts(body),
                    }];
                }

                // Header has statements that must run every iteration before condition check
                // Emit: while (true) { header_stmts; if (!cond) { body } }
                let mut loop_stmts = header_stmts;
                if !body.is_empty() {
                    loop_stmts.push(Stmt::If {
                        cond: negate(cond.clone()),
                        then_block: Block::with_stmts(body),
                        else_block: None,
                    });
                }

                // Use While with true condition for infinite loop (no break at end)
                return vec![Stmt::While {
                    cond: Expr::i32_const(1),
                    body: Block::with_stmts(loop_stmts),
                }];
            }

            // Pattern: if (!cond) continue; body...
            // else_target = header (continue), then_target = body
            if else_is_header && in_loop_then && !then_is_header {
                let body = emit_from_in_loop(ctx, *then_target, Some(header), loop_body.clone());

                // If header has no statements, we can simplify to while(cond) { body }
                if header_stmts.is_empty() {
                    return vec![Stmt::While {
                        cond: cond.clone(),
                        body: Block::with_stmts(body),
                    }];
                }

                // Header has statements - emit as loop with conditional
                let mut loop_stmts = header_stmts;
                if !body.is_empty() {
                    loop_stmts.push(Stmt::If {
                        cond: cond.clone(),
                        then_block: Block::with_stmts(body),
                        else_block: None,
                    });
                }

                // Use While with true condition for infinite loop (no break at end)
                return vec![Stmt::While {
                    cond: Expr::i32_const(1),
                    body: Block::with_stmts(loop_stmts),
                }];
            }

            match (in_loop_then, in_loop_else) {
                // then exits, else is body -> do { body } while (!cond)
                // WASM: body runs first, then "if (cond) exit; else continue"
                (false, true) => {
                    let body = emit_from_in_loop(ctx, *else_target, Some(header), loop_body.clone());
                    let mut full_body = header_stmts;
                    full_body.extend(body);

                    // Use do-while: body runs before condition check
                    vec![Stmt::DoWhile {
                        body: Block::with_stmts(full_body),
                        cond: negate(cond.clone()),
                    }]
                }
                // then is body (continue), else exits -> do { body } while (cond)
                // WASM: body runs first, then "if (cond) continue; else exit"
                (true, false) => {
                    let body = emit_from_in_loop(ctx, *then_target, Some(header), loop_body.clone());
                    let mut full_body = header_stmts;
                    full_body.extend(body);

                    // Use do-while: body runs before condition check
                    vec![Stmt::DoWhile {
                        body: Block::with_stmts(full_body),
                        cond: cond.clone(),
                    }]
                }
                // Both in loop - infinite loop with conditional break
                (true, true) => {
                    emit_infinite_loop(ctx, header, &header_stmts, &loop_body, cond, *then_target, *else_target, true)
                }
                // Both exit - shouldn't happen for a real loop, emit as if
                (false, false) => {
                    let then_stmts = emit_from(ctx, *then_target, None);
                    let else_stmts = emit_from(ctx, *else_target, None);
                    vec![Stmt::If {
                        cond: cond.clone(),
                        then_block: Block::with_stmts(then_stmts),
                        else_block: if else_stmts.is_empty() {
                            None
                        } else {
                            Some(Block::with_stmts(else_stmts))
                        },
                    }]
                }
            }
        }

        Terminator::Fallthrough(next) | Terminator::Goto(next) => {
            // Unconditional loop entry
            if ctx.is_in_loop(header, *next) {
                let body = emit_from_in_loop(ctx, *next, Some(header), loop_body.clone());
                let mut full_body = header_stmts;
                full_body.extend(body);

                // Check if body ends with a conditional that could be do-while
                if let Some(last) = full_body.last() {
                    if let Stmt::If { cond: _, then_block, else_block: None } = last {
                        if then_block.stmts.is_empty() || matches!(then_block.stmts.last(), Some(Stmt::Br { .. })) {
                            // Might be do-while pattern
                            // For now just emit as loop
                        }
                    }
                }

                // Emit as infinite loop (using While true so no break is added)
                vec![Stmt::While {
                    cond: Expr::i32_const(1),
                    body: Block::with_stmts(full_body),
                }]
            } else {
                // Next is outside loop - weird case
                header_stmts
            }
        }

        _ => header_stmts,
    }
}

/// Emit infinite loop with break condition inside
fn emit_infinite_loop(
    ctx: &mut EmitCtx,
    header: NodeId,
    header_stmts: &[Stmt],
    loop_body_set: &HashSet<NodeId>,
    cond: &Expr,
    then_target: NodeId,
    else_target: NodeId,
    then_in_loop: bool,
) -> Vec<Stmt> {
    let mut body = header_stmts.to_vec();

    // Add break condition
    // If condition is true and then_target exits, break
    // Otherwise, condition false and else_target exits, break
    let (continue_target, break_cond) = if then_in_loop {
        // then continues, else might exit (but both are in loop here)
        // Just emit both branches inside loop
        (then_target, None)
    } else {
        (else_target, Some(cond.clone()))
    };

    if let Some(bc) = break_cond {
        body.push(Stmt::If {
            cond: bc,
            then_block: Block::with_stmts(vec![Stmt::Br { label: u32::MAX, is_loop: false }]),
            else_block: None,
        });
    }

    // Emit body from continue target
    if !ctx.emitted.contains(&continue_target) {
        let inner = emit_from_in_loop(ctx, continue_target, Some(header), loop_body_set.clone());
        body.extend(inner);
    }

    // Use While(true) for infinite loop that can be broken out of
    vec![Stmt::While {
        cond: Expr::i32_const(1),
        body: Block::with_stmts(body),
    }]
}

/// Find where two branches merge (earliest common node in control flow)
fn find_merge(cfg: &Cfg, a: NodeId, b: NodeId, stop: Option<NodeId>) -> Option<NodeId> {
    // Use BFS to find distances from both start points
    let dist_a = bfs_distances(cfg, a, 20);
    let dist_b = bfs_distances(cfg, b, 20);

    // Prefer stop point if reachable
    if let Some(s) = stop {
        if dist_a.contains_key(&s) || dist_b.contains_key(&s) {
            return Some(s);
        }
    }

    // Find common node with minimum max distance (earliest merge point)
    let mut best: Option<(NodeId, usize)> = None;
    for (&node, &da) in &dist_a {
        if let Some(&db) = dist_b.get(&node) {
            let max_dist = da.max(db);
            match best {
                None => best = Some((node, max_dist)),
                Some((_, best_dist)) if max_dist < best_dist => best = Some((node, max_dist)),
                _ => {}
            }
        }
    }

    best.map(|(n, _)| n).or(stop)
}

/// BFS to compute distances from start node
fn bfs_distances(cfg: &Cfg, start: NodeId, limit: usize) -> HashMap<NodeId, usize> {
    use std::collections::VecDeque;
    let mut dist = HashMap::new();
    let mut queue = VecDeque::new();

    dist.insert(start, 0);
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        let d = dist[&node];
        if d >= limit {
            continue;
        }
        for &succ in cfg.get_successors(node) {
            if !dist.contains_key(&succ) {
                dist.insert(succ, d + 1);
                queue.push_back(succ);
            }
        }
    }

    dist
}

/// Find loop exit node
fn find_loop_exit(ctx: &EmitCtx, header: NodeId) -> Option<NodeId> {
    let body = ctx.loop_body(header)?;
    for &node in body {
        for succ in ctx.cfg.get_successors(node) {
            if !body.contains(succ) {
                return Some(*succ);
            }
        }
    }
    None
}

/// Get reachable nodes within depth limit
fn reachable(cfg: &Cfg, start: NodeId, limit: usize) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    let mut stack = vec![(start, 0)];

    while let Some((n, d)) = stack.pop() {
        if d > limit || result.contains(&n) {
            continue;
        }
        result.insert(n);
        for s in cfg.get_successors(n) {
            stack.push((*s, d + 1));
        }
    }

    result
}

/// Negate condition
fn negate(cond: Expr) -> Expr {
    match cond.kind {
        ExprKind::Compare(op, a, b) => {
            let neg_op = match op {
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
            Expr::with_type(ExprKind::Compare(neg_op, a, b), InferredType::Bool)
        }
        ExprKind::UnaryOp(UnaryOp::Eqz, inner) => *inner,
        _ => Expr::with_type(
            ExprKind::UnaryOp(UnaryOp::Eqz, Box::new(cond)),
            InferredType::Bool,
        ),
    }
}
