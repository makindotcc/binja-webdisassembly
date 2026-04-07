//! CFG construction from IR
//!
//! Converts WASM IR blocks into a control flow graph.

use std::collections::HashMap;

use super::{Cfg, CfgNode, NodeId, Terminator};
use crate::ir::{Block, Stmt};

/// Build a CFG from an IR block
pub fn build_cfg(block: &Block) -> Cfg {
    let mut builder = CfgBuilder::new();
    builder.build(block);
    builder.cfg
}

/// Label target information
#[derive(Debug, Clone, Copy)]
struct LabelTarget {
    /// The node to jump to when breaking/continuing
    node: NodeId,
    /// Whether this is a loop (br continues) or block (br breaks)
    #[allow(dead_code)]
    is_loop: bool,
}

/// CFG builder state
struct CfgBuilder {
    cfg: Cfg,
    /// Maps WASM block/loop labels to their targets
    label_targets: HashMap<u32, LabelTarget>,
    /// Stack of active labels for nested blocks
    label_stack: Vec<(u32, LabelTarget)>,
}

impl CfgBuilder {
    fn new() -> Self {
        Self {
            cfg: Cfg::new(),
            label_targets: HashMap::new(),
            label_stack: Vec::new(),
        }
    }

    fn build(&mut self, block: &Block) {
        // Create entry node
        let entry = self.cfg.add_node(CfgNode::new(0));
        self.cfg.entry = entry;

        // Create exit node (for returns)
        let exit = self.cfg.add_node(CfgNode::new(1));
        self.cfg.exit = exit;

        // Process the block
        let current = self.process_block(block, entry, exit);

        // Connect final node to exit if it doesn't already terminate
        if let Some(last) = current {
            if let Some(node) = self.cfg.get_node(last) {
                if matches!(node.terminator, Terminator::Unreachable) {
                    if let Some(node) = self.cfg.get_node_mut(last) {
                        node.terminator = Terminator::Fallthrough(exit);
                    }
                    self.cfg.add_edge(last, exit);
                }
            }
        }
    }

    /// Process a block of statements, returning the last active node ID
    fn process_block(
        &mut self,
        block: &Block,
        mut current: NodeId,
        exit: NodeId,
    ) -> Option<NodeId> {
        for stmt in &block.stmts {
            match stmt {
                // Control flow statements
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    let then_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    let else_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    let merge_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));

                    // Set branch terminator
                    if let Some(node) = self.cfg.get_node_mut(current) {
                        node.terminator = Terminator::Branch {
                            cond: cond.clone(),
                            then_target: then_node,
                            else_target: else_node,
                        };
                    }
                    self.cfg.add_edge(current, then_node);
                    self.cfg.add_edge(current, else_node);

                    // Process then branch
                    let then_end = self.process_block(then_block, then_node, exit);
                    if let Some(end) = then_end {
                        self.connect_to_merge(end, merge_node);
                    }

                    // Process else branch
                    if let Some(else_blk) = else_block {
                        let else_end = self.process_block(else_blk, else_node, exit);
                        if let Some(end) = else_end {
                            self.connect_to_merge(end, merge_node);
                        }
                    } else {
                        self.connect_to_merge(else_node, merge_node);
                    }

                    current = merge_node;
                }

                Stmt::Block { label, body } => {
                    // Create continuation node (where break jumps to)
                    let block_exit = self.cfg.add_node(CfgNode::new(self.cfg.len()));

                    // Register label target (break goes to block_exit)
                    let target = LabelTarget {
                        node: block_exit,
                        is_loop: false,
                    };
                    self.label_targets.insert(*label, target);
                    self.label_stack.push((*label, target));

                    // Process body
                    let body_end = self.process_block(body, current, exit);

                    // Pop label
                    self.label_stack.pop();
                    self.label_targets.remove(label);

                    // Connect body end to block exit
                    if let Some(end) = body_end {
                        self.connect_to_merge(end, block_exit);
                    }

                    current = block_exit;
                }

                Stmt::Loop { label, body } => {
                    // Create header node for the loop
                    let header = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    if let Some(node) = self.cfg.get_node_mut(header) {
                        node.label = Some(*label);
                    }

                    // Create exit node
                    let loop_exit = self.cfg.add_node(CfgNode::new(self.cfg.len()));

                    // Connect current to header
                    self.connect_fallthrough(current, header);

                    // Register label target (br goes back to header for loops)
                    let target = LabelTarget {
                        node: header,
                        is_loop: true,
                    };
                    self.label_targets.insert(*label, target);
                    self.label_stack.push((*label, target));

                    // Process body
                    let body_end = self.process_block(body, header, exit);

                    // Pop label
                    self.label_stack.pop();
                    self.label_targets.remove(label);

                    // In WASM, falling through the end of a loop body EXITS the loop
                    // (explicit br $loop is needed to continue)
                    if let Some(end) = body_end {
                        if let Some(node) = self.cfg.get_node(end) {
                            if matches!(node.terminator, Terminator::Unreachable) {
                                if let Some(node) = self.cfg.get_node_mut(end) {
                                    node.terminator = Terminator::Fallthrough(loop_exit);
                                }
                                self.cfg.add_edge(end, loop_exit);
                            }
                        }
                    }

                    current = loop_exit;
                }

                Stmt::While { cond, body } => {
                    let header = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    let body_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    let loop_exit = self.cfg.add_node(CfgNode::new(self.cfg.len()));

                    self.connect_fallthrough(current, header);

                    if let Some(node) = self.cfg.get_node_mut(header) {
                        node.terminator = Terminator::Branch {
                            cond: cond.clone(),
                            then_target: body_node,
                            else_target: loop_exit,
                        };
                    }
                    self.cfg.add_edge(header, body_node);
                    self.cfg.add_edge(header, loop_exit);

                    let body_end = self.process_block(body, body_node, exit);
                    if let Some(end) = body_end {
                        self.connect_goto(end, header);
                    }

                    current = loop_exit;
                }

                Stmt::DoWhile { body, cond } => {
                    let body_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    let loop_exit = self.cfg.add_node(CfgNode::new(self.cfg.len()));

                    self.connect_fallthrough(current, body_node);

                    let body_end = self.process_block(body, body_node, exit);
                    if let Some(end) = body_end {
                        if let Some(node) = self.cfg.get_node_mut(end) {
                            node.terminator = Terminator::Branch {
                                cond: cond.clone(),
                                then_target: body_node,
                                else_target: loop_exit,
                            };
                        }
                        self.cfg.add_edge(end, body_node);
                        self.cfg.add_edge(end, loop_exit);
                    }

                    current = loop_exit;
                }

                Stmt::Br { label, .. } => {
                    if let Some(target) = self.label_targets.get(label) {
                        self.connect_goto(current, target.node);
                    }
                    // After unconditional branch, create unreachable node
                    let new_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    current = new_node;
                }

                Stmt::BrIf { label, cond, .. } => {
                    if let Some(target) = self.label_targets.get(label) {
                        let continue_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));

                        if let Some(node) = self.cfg.get_node_mut(current) {
                            node.terminator = Terminator::Branch {
                                cond: cond.clone(),
                                then_target: target.node,
                                else_target: continue_node,
                            };
                        }
                        self.cfg.add_edge(current, target.node);
                        self.cfg.add_edge(current, continue_node);

                        current = continue_node;
                    }
                }

                Stmt::BrTable {
                    index,
                    targets,
                    default,
                } => {
                    let mut target_nodes = Vec::new();

                    for target in targets {
                        if let Some(t) = self.label_targets.get(&target.label) {
                            target_nodes.push(t.node);
                            self.cfg.add_edge(current, t.node);
                        }
                    }

                    let default_node = if let Some(t) = self.label_targets.get(&default.label) {
                        self.cfg.add_edge(current, t.node);
                        t.node
                    } else {
                        let node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                        self.cfg.add_edge(current, node);
                        node
                    };

                    if let Some(node) = self.cfg.get_node_mut(current) {
                        node.terminator = Terminator::Switch {
                            index: index.clone(),
                            targets: target_nodes,
                            default: default_node,
                        };
                    }

                    let new_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    current = new_node;
                }

                Stmt::Return(value) => {
                    if let Some(node) = self.cfg.get_node_mut(current) {
                        node.terminator = Terminator::Return(value.clone());
                    }
                    let new_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    current = new_node;
                }

                Stmt::Unreachable => {
                    if let Some(node) = self.cfg.get_node_mut(current) {
                        node.terminator = Terminator::Unreachable;
                    }
                    let new_node = self.cfg.add_node(CfgNode::new(self.cfg.len()));
                    current = new_node;
                }

                // Non-control-flow statements
                Stmt::LocalSet { .. }
                | Stmt::GlobalSet { .. }
                | Stmt::Store { .. }
                | Stmt::Expr(_)
                | Stmt::Drop(_)
                | Stmt::Nop => {
                    if let Some(node) = self.cfg.get_node_mut(current) {
                        node.stmts.push(stmt.clone());
                    }
                }

                Stmt::Switch { .. } => {
                    // Switch is a higher-level construct, treat as opaque statement
                    if let Some(node) = self.cfg.get_node_mut(current) {
                        node.stmts.push(stmt.clone());
                    }
                }
            }
        }

        Some(current)
    }

    fn connect_fallthrough(&mut self, from: NodeId, to: NodeId) {
        if let Some(node) = self.cfg.get_node_mut(from) {
            if matches!(node.terminator, Terminator::Unreachable) {
                node.terminator = Terminator::Fallthrough(to);
            }
        }
        self.cfg.add_edge(from, to);
    }

    fn connect_goto(&mut self, from: NodeId, to: NodeId) {
        if let Some(node) = self.cfg.get_node_mut(from) {
            if matches!(node.terminator, Terminator::Unreachable) {
                node.terminator = Terminator::Goto(to);
            }
        }
        self.cfg.add_edge(from, to);
    }

    fn connect_to_merge(&mut self, from: NodeId, merge: NodeId) {
        if let Some(node) = self.cfg.get_node(from) {
            if matches!(node.terminator, Terminator::Unreachable) {
                if let Some(node) = self.cfg.get_node_mut(from) {
                    node.terminator = Terminator::Fallthrough(merge);
                }
                self.cfg.add_edge(from, merge);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Expr;

    #[test]
    fn test_empty_block() {
        let block = Block { stmts: vec![] };
        let cfg = build_cfg(&block);
        assert!(!cfg.is_empty());
        assert_eq!(cfg.entry, 0);
    }

    #[test]
    fn test_simple_if() {
        let block = Block {
            stmts: vec![Stmt::If {
                cond: Expr::i32_const(1),
                then_block: Block { stmts: vec![] },
                else_block: None,
            }],
        };
        let cfg = build_cfg(&block);
        assert!(cfg.len() >= 4); // entry, then, else, merge, exit
    }

    #[test]
    fn test_simple_loop() {
        let block = Block {
            stmts: vec![Stmt::Loop {
                label: 0,
                body: Block {
                    stmts: vec![Stmt::Br {
                        label: 0,
                        is_loop: true,
                    }],
                },
            }],
        };
        let cfg = build_cfg(&block);
        assert!(cfg.len() >= 3);
    }
}
