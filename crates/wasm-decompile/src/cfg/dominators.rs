//! Dominator analysis for CFG
//!
//! Computes dominator tree and provides dominator queries.

use std::collections::{HashMap, HashSet};

use super::{Cfg, NodeId};

/// Dominator information for a CFG
#[derive(Debug, Clone)]
pub struct DominatorInfo {
    /// Immediate dominator for each node
    pub idom: HashMap<NodeId, NodeId>,
    /// Dominator set for each node (all nodes that dominate it)
    pub dominators: HashMap<NodeId, HashSet<NodeId>>,
    /// Nodes dominated by each node
    pub dominated: HashMap<NodeId, HashSet<NodeId>>,
    /// Post-order numbering
    pub post_order: Vec<NodeId>,
}

impl DominatorInfo {
    /// Check if `a` dominates `b`
    pub fn dominates(&self, a: NodeId, b: NodeId) -> bool {
        if a == b {
            return true;
        }
        self.dominators
            .get(&b)
            .map(|doms| doms.contains(&a))
            .unwrap_or(false)
    }

    /// Check if `a` strictly dominates `b` (a dominates b and a != b)
    pub fn strictly_dominates(&self, a: NodeId, b: NodeId) -> bool {
        a != b && self.dominates(a, b)
    }

    /// Get immediate dominator of a node
    pub fn get_idom(&self, node: NodeId) -> Option<NodeId> {
        self.idom.get(&node).copied()
    }

    /// Get all nodes dominated by a given node
    pub fn get_dominated(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.dominated
            .get(&node)
            .into_iter()
            .flat_map(|s| s.iter().copied())
    }
}

/// Compute dominators using iterative dataflow algorithm
pub fn compute_dominators(cfg: &Cfg) -> DominatorInfo {
    if cfg.is_empty() {
        return DominatorInfo {
            idom: HashMap::new(),
            dominators: HashMap::new(),
            dominated: HashMap::new(),
            post_order: Vec::new(),
        };
    }

    let entry = cfg.entry;
    let nodes: Vec<NodeId> = (0..cfg.len()).collect();

    // Compute post-order
    let post_order = compute_post_order(cfg);

    // Initialize dominator sets
    let mut dominators: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
    let all_nodes: HashSet<NodeId> = nodes.iter().copied().collect();

    for &node in &nodes {
        if node == entry {
            let mut entry_dom = HashSet::new();
            entry_dom.insert(entry);
            dominators.insert(entry, entry_dom);
        } else {
            dominators.insert(node, all_nodes.clone());
        }
    }

    // Iterate until fixed point
    let mut changed = true;
    while changed {
        changed = false;

        for &node in &nodes {
            if node == entry {
                continue;
            }

            let preds: Vec<NodeId> = cfg.get_predecessors(node).collect();

            if preds.is_empty() {
                let mut new_dom = HashSet::new();
                new_dom.insert(node);
                if dominators.get(&node) != Some(&new_dom) {
                    dominators.insert(node, new_dom);
                    changed = true;
                }
                continue;
            }

            let mut new_dom: Option<HashSet<NodeId>> = None;
            for &pred in &preds {
                if let Some(pred_dom) = dominators.get(&pred) {
                    new_dom = Some(match new_dom {
                        None => pred_dom.clone(),
                        Some(current) => current.intersection(pred_dom).copied().collect(),
                    });
                }
            }

            let mut new_dom = new_dom.unwrap_or_default();
            new_dom.insert(node);

            if dominators.get(&node) != Some(&new_dom) {
                dominators.insert(node, new_dom);
                changed = true;
            }
        }
    }

    // Extract immediate dominators
    let idom = extract_idom(&dominators, &nodes, entry);

    // Build dominated sets
    let mut dominated: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
    for &node in &nodes {
        dominated.insert(node, HashSet::new());
    }

    for (&node, doms) in &dominators {
        for &dom in doms {
            if dom != node {
                dominated.entry(dom).or_default().insert(node);
            }
        }
    }

    DominatorInfo {
        idom,
        dominators,
        dominated,
        post_order,
    }
}

/// Compute post-order traversal
fn compute_post_order(cfg: &Cfg) -> Vec<NodeId> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();

    fn dfs(cfg: &Cfg, node: NodeId, visited: &mut HashSet<NodeId>, order: &mut Vec<NodeId>) {
        if visited.contains(&node) {
            return;
        }
        visited.insert(node);

        for succ in cfg.get_successors(node) {
            dfs(cfg, *succ, visited, order);
        }

        order.push(node);
    }

    dfs(cfg, cfg.entry, &mut visited, &mut order);
    order
}

/// Extract immediate dominators from dominator sets
fn extract_idom(
    dominators: &HashMap<NodeId, HashSet<NodeId>>,
    nodes: &[NodeId],
    entry: NodeId,
) -> HashMap<NodeId, NodeId> {
    let mut idom = HashMap::new();

    for &node in nodes {
        if node == entry {
            continue;
        }

        if let Some(doms) = dominators.get(&node) {
            let strict_doms: Vec<NodeId> = doms.iter().copied().filter(|&d| d != node).collect();

            if strict_doms.is_empty() {
                continue;
            }

            // Find the dominator closest to node (dominated by all others)
            for &candidate in &strict_doms {
                let is_idom = strict_doms.iter().all(|&other| {
                    if other == candidate {
                        true
                    } else if let Some(other_doms) = dominators.get(&candidate) {
                        other_doms.contains(&other)
                    } else {
                        false
                    }
                });

                if is_idom {
                    idom.insert(node, candidate);
                    break;
                }
            }
        }
    }

    idom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{CfgNode, Terminator};

    #[test]
    fn test_linear_cfg() {
        let mut cfg = Cfg::new();
        let a = cfg.add_node(CfgNode::new(0));
        let b = cfg.add_node(CfgNode::new(1));
        let c = cfg.add_node(CfgNode::new(2));

        cfg.entry = a;
        cfg.get_node_mut(a).unwrap().terminator = Terminator::Fallthrough(b);
        cfg.get_node_mut(b).unwrap().terminator = Terminator::Fallthrough(c);
        cfg.add_edge(a, b);
        cfg.add_edge(b, c);

        let dom = compute_dominators(&cfg);

        assert!(dom.dominates(a, a));
        assert!(dom.dominates(a, b));
        assert!(dom.dominates(a, c));
        assert!(dom.dominates(b, c));
        assert!(!dom.dominates(c, b));
    }

    #[test]
    fn test_diamond_cfg() {
        let mut cfg = Cfg::new();
        let a = cfg.add_node(CfgNode::new(0));
        let b = cfg.add_node(CfgNode::new(1));
        let c = cfg.add_node(CfgNode::new(2));
        let d = cfg.add_node(CfgNode::new(3));

        cfg.entry = a;
        cfg.add_edge(a, b);
        cfg.add_edge(a, c);
        cfg.add_edge(b, d);
        cfg.add_edge(c, d);

        let dom = compute_dominators(&cfg);

        assert!(dom.dominates(a, d));
        assert!(!dom.dominates(b, d));
        assert!(!dom.dominates(c, d));
        assert_eq!(dom.get_idom(d), Some(a));
    }
}
