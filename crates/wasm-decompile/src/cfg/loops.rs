//! Loop analysis for CFG
//!
//! Detects natural loops using back edge analysis.

use std::collections::{HashMap, HashSet, VecDeque};

use super::dominators::DominatorInfo;
use super::{Cfg, EdgeKind, NodeId};

/// Information about loops in the CFG
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// All detected natural loops
    pub loops: Vec<NaturalLoop>,
    /// Map from loop header to loop index
    pub header_to_loop: HashMap<NodeId, usize>,
    /// Map from node to innermost containing loop
    pub node_to_loop: HashMap<NodeId, usize>,
    /// Edge classifications
    pub edge_kinds: HashMap<(NodeId, NodeId), EdgeKind>,
}

impl LoopInfo {
    /// Check if a node is a loop header
    pub fn is_loop_header(&self, node: NodeId) -> bool {
        self.header_to_loop.contains_key(&node)
    }

    /// Get the loop containing a node (if any)
    pub fn get_containing_loop(&self, node: NodeId) -> Option<&NaturalLoop> {
        self.node_to_loop
            .get(&node)
            .and_then(|&idx| self.loops.get(idx))
    }

    /// Check if an edge is a back edge
    pub fn is_back_edge(&self, from: NodeId, to: NodeId) -> bool {
        self.edge_kinds
            .get(&(from, to))
            .map(|&k| k == EdgeKind::Back)
            .unwrap_or(false)
    }
}

/// A natural loop in the CFG
#[derive(Debug, Clone)]
pub struct NaturalLoop {
    /// Loop header (entry point, target of back edges)
    pub header: NodeId,
    /// All nodes in the loop body (including header)
    pub body: HashSet<NodeId>,
    /// Back edges (tail -> header)
    pub back_edges: Vec<(NodeId, NodeId)>,
    /// Exit edges (node in loop -> node outside loop)
    pub exit_edges: Vec<(NodeId, NodeId)>,
    /// Nodes that exit the loop
    pub exiting_nodes: HashSet<NodeId>,
    /// Parent loop index (for nested loops)
    pub parent: Option<usize>,
    /// Child loop indices
    pub children: Vec<usize>,
}

impl NaturalLoop {
    /// Check if this loop contains a node
    pub fn contains(&self, node: NodeId) -> bool {
        self.body.contains(&node)
    }

    /// Get the single exit node if this loop has exactly one exit
    pub fn single_exit(&self) -> Option<NodeId> {
        let exits: HashSet<_> = self.exit_edges.iter().map(|&(_, to)| to).collect();
        if exits.len() == 1 {
            exits.into_iter().next()
        } else {
            None
        }
    }
}

/// Analyze loops in the CFG
pub fn analyze_loops(cfg: &Cfg, dom: &DominatorInfo) -> LoopInfo {
    let mut info = LoopInfo {
        loops: Vec::new(),
        header_to_loop: HashMap::new(),
        node_to_loop: HashMap::new(),
        edge_kinds: HashMap::new(),
    };

    if cfg.is_empty() {
        return info;
    }

    // Step 1: Classify edges
    classify_edges(cfg, dom, &mut info);

    // Step 2: Find back edges
    let back_edges: Vec<_> = info
        .edge_kinds
        .iter()
        .filter(|&(_, &kind)| kind == EdgeKind::Back)
        .map(|(&edge, _)| edge)
        .collect();

    // Group back edges by header
    let mut header_back_edges: HashMap<NodeId, Vec<(NodeId, NodeId)>> = HashMap::new();
    for (tail, header) in back_edges {
        header_back_edges
            .entry(header)
            .or_default()
            .push((tail, header));
    }

    // Step 3: Create natural loops
    for (header, back_edges) in header_back_edges {
        let body = find_natural_loop_body(cfg, header, &back_edges);
        let exit_edges = find_exit_edges(cfg, &body);
        let exiting_nodes: HashSet<_> = exit_edges.iter().map(|&(from, _)| from).collect();

        let loop_idx = info.loops.len();
        info.loops.push(NaturalLoop {
            header,
            body,
            back_edges,
            exit_edges,
            exiting_nodes,
            parent: None,
            children: Vec::new(),
        });
        info.header_to_loop.insert(header, loop_idx);
    }

    // Step 4: Determine loop nesting
    determine_loop_nesting(&mut info);

    // Step 5: Map nodes to innermost loops
    for (loop_idx, lp) in info.loops.iter().enumerate() {
        for &node in &lp.body {
            let should_update = match info.node_to_loop.get(&node) {
                None => true,
                Some(&existing_idx) => info.loops[existing_idx].body.len() > lp.body.len(),
            };
            if should_update {
                info.node_to_loop.insert(node, loop_idx);
            }
        }
    }

    info
}

/// Classify edges using DFS
fn classify_edges(cfg: &Cfg, dom: &DominatorInfo, info: &mut LoopInfo) {
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();
    let mut dfs_num = HashMap::new();
    let mut counter = 0;

    dfs_classify(
        cfg,
        dom,
        cfg.entry,
        &mut visited,
        &mut in_stack,
        &mut dfs_num,
        &mut counter,
        info,
    );
}

fn dfs_classify(
    cfg: &Cfg,
    dom: &DominatorInfo,
    node: NodeId,
    visited: &mut HashSet<NodeId>,
    in_stack: &mut HashSet<NodeId>,
    dfs_num: &mut HashMap<NodeId, usize>,
    counter: &mut usize,
    info: &mut LoopInfo,
) {
    visited.insert(node);
    in_stack.insert(node);
    dfs_num.insert(node, *counter);
    *counter += 1;

    for succ in cfg.get_successors(node).iter().copied() {
        let kind = if !visited.contains(&succ) {
            dfs_classify(cfg, dom, succ, visited, in_stack, dfs_num, counter, info);
            EdgeKind::Tree
        } else if in_stack.contains(&succ) {
            // Back edge - verify with dominators
            if dom.dominates(succ, node) {
                EdgeKind::Back
            } else {
                EdgeKind::Back // Still a back edge in DFS terms
            }
        } else if dfs_num.get(&node) < dfs_num.get(&succ) {
            EdgeKind::Forward
        } else {
            EdgeKind::Cross
        };

        info.edge_kinds.insert((node, succ), kind);
    }

    in_stack.remove(&node);
}

/// Find the body of a natural loop
fn find_natural_loop_body(
    cfg: &Cfg,
    header: NodeId,
    back_edges: &[(NodeId, NodeId)],
) -> HashSet<NodeId> {
    let mut body = HashSet::new();
    body.insert(header);

    let mut worklist: VecDeque<NodeId> = VecDeque::new();
    for &(tail, _) in back_edges {
        if !body.contains(&tail) {
            body.insert(tail);
            worklist.push_back(tail);
        }
    }

    while let Some(node) = worklist.pop_front() {
        for pred in cfg.get_predecessors(node) {
            if !body.contains(&pred) {
                body.insert(pred);
                worklist.push_back(pred);
            }
        }
    }

    body
}

/// Find edges that exit the loop
fn find_exit_edges(cfg: &Cfg, body: &HashSet<NodeId>) -> Vec<(NodeId, NodeId)> {
    let mut exits = Vec::new();

    for &node in body {
        for succ in cfg.get_successors(node).iter().copied() {
            if !body.contains(&succ) {
                exits.push((node, succ));
            }
        }
    }

    exits
}

/// Determine loop nesting relationships
fn determine_loop_nesting(info: &mut LoopInfo) {
    let n = info.loops.len();

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }

            if info.loops[i].body.is_superset(&info.loops[j].body) {
                let j_header = info.loops[j].header;
                if info.loops[i].body.contains(&j_header) {
                    let should_update = match info.loops[j].parent {
                        None => true,
                        Some(p) => info.loops[p].body.len() > info.loops[i].body.len(),
                    };
                    if should_update {
                        if let Some(old_parent) = info.loops[j].parent {
                            info.loops[old_parent].children.retain(|&c| c != j);
                        }
                        info.loops[j].parent = Some(i);
                        info.loops[i].children.push(j);
                    }
                }
            }
        }
    }

    for lp in &mut info.loops {
        lp.children.sort();
        lp.children.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{CfgNode, Terminator};
    use crate::cfg::dominators::compute_dominators;

    #[test]
    fn test_simple_loop() {
        let mut cfg = Cfg::new();
        let a = cfg.add_node(CfgNode::new(0));
        let b = cfg.add_node(CfgNode::new(1));

        cfg.entry = a;
        cfg.add_edge(a, b);
        cfg.add_edge(b, a);

        if let Some(node) = cfg.get_node_mut(a) {
            node.terminator = Terminator::Fallthrough(b);
        }
        if let Some(node) = cfg.get_node_mut(b) {
            node.terminator = Terminator::Goto(a);
        }

        let dom = compute_dominators(&cfg);
        let loops = analyze_loops(&cfg, &dom);

        assert_eq!(loops.loops.len(), 1);
        assert!(loops.is_loop_header(a));
    }

    #[test]
    fn test_while_loop() {
        let mut cfg = Cfg::new();
        let header = cfg.add_node(CfgNode::new(0));
        let body = cfg.add_node(CfgNode::new(1));
        let exit = cfg.add_node(CfgNode::new(2));

        cfg.entry = header;
        cfg.add_edge(header, body);
        cfg.add_edge(header, exit);
        cfg.add_edge(body, header);

        let dom = compute_dominators(&cfg);
        let loops = analyze_loops(&cfg, &dom);

        assert_eq!(loops.loops.len(), 1);
        assert!(loops.is_loop_header(header));
        assert!(loops.loops[0].body.contains(&body));
        assert!(!loops.loops[0].body.contains(&exit));
    }
}
