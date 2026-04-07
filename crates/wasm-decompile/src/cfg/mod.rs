//! Control Flow Graph structures and algorithms
//!
//! This module provides:
//! - CFG data structures (`Cfg`, `CfgNode`, `Terminator`)
//! - CFG construction from IR (`build_cfg`)
//! - Dominator analysis (`compute_dominators`)
//! - Loop detection (`analyze_loops`)

pub mod build;
pub mod dominators;
pub mod loops;

use std::collections::{HashMap, HashSet};

use crate::ir::{Expr, Stmt};

// Re-exports
pub use dominators::DominatorInfo;
pub use loops::{LoopInfo, NaturalLoop};

/// Unique identifier for a CFG node
pub type NodeId = usize;

/// Control Flow Graph
#[derive(Debug, Clone)]
pub struct Cfg {
    /// All nodes in the CFG
    pub nodes: Vec<CfgNode>,
    /// Entry node ID
    pub entry: NodeId,
    /// Exit node ID (for returns)
    pub exit: NodeId,
    /// Predecessor edges: node -> set of predecessors
    pub predecessors: HashMap<NodeId, HashSet<NodeId>>,
    /// Successor edges: node -> list of successors
    pub successors: HashMap<NodeId, Vec<NodeId>>,
}

impl Cfg {
    /// Create a new empty CFG
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            entry: 0,
            exit: 0,
            predecessors: HashMap::new(),
            successors: HashMap::new(),
        }
    }

    /// Add a node to the CFG and return its ID
    pub fn add_node(&mut self, node: CfgNode) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        self.predecessors.insert(id, HashSet::new());
        self.successors.insert(id, Vec::new());
        id
    }

    /// Add an edge from `from` to `to`
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        self.successors.entry(from).or_default().push(to);
        self.predecessors.entry(to).or_default().insert(from);
    }

    /// Get successors of a node
    pub fn get_successors(&self, node: NodeId) -> &[NodeId] {
        self.successors.get(&node).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get predecessors of a node
    pub fn get_predecessors(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.predecessors
            .get(&node)
            .into_iter()
            .flat_map(|s| s.iter().copied())
    }

    /// Get number of nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if CFG is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get a node by ID
    pub fn get_node(&self, id: NodeId) -> Option<&CfgNode> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut CfgNode> {
        self.nodes.get_mut(id)
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the CFG
#[derive(Debug, Clone)]
pub struct CfgNode {
    /// Unique identifier
    pub id: NodeId,
    /// Statements in this basic block
    pub stmts: Vec<Stmt>,
    /// How this block terminates
    pub terminator: Terminator,
    /// Immediate dominator (set by dominator analysis)
    pub idom: Option<NodeId>,
    /// Original WASM block/loop label if applicable
    pub label: Option<u32>,
}

impl CfgNode {
    /// Create a new CFG node
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            stmts: Vec::new(),
            terminator: Terminator::Unreachable,
            idom: None,
            label: None,
        }
    }

    /// Create a node with statements
    pub fn with_stmts(id: NodeId, stmts: Vec<Stmt>) -> Self {
        Self {
            id,
            stmts,
            terminator: Terminator::Unreachable,
            idom: None,
            label: None,
        }
    }
}

/// How a basic block terminates
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Fall through to next node
    Fallthrough(NodeId),
    /// Unconditional jump
    Goto(NodeId),
    /// Conditional branch
    Branch {
        cond: Expr,
        then_target: NodeId,
        else_target: NodeId,
    },
    /// Switch/branch table
    Switch {
        index: Expr,
        targets: Vec<NodeId>,
        default: NodeId,
    },
    /// Return from function
    Return(Option<Expr>),
    /// Unreachable code
    Unreachable,
}

impl Terminator {
    /// Get all successor node IDs
    pub fn successors(&self) -> Vec<NodeId> {
        match self {
            Terminator::Fallthrough(n) | Terminator::Goto(n) => vec![*n],
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => vec![*then_target, *else_target],
            Terminator::Switch { targets, default, .. } => {
                let mut succs: Vec<_> = targets.clone();
                succs.push(*default);
                succs
            }
            Terminator::Return(_) | Terminator::Unreachable => vec![],
        }
    }
}

/// Edge classification for DFS traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Tree edge (part of DFS tree)
    Tree,
    /// Back edge (target dominates source - indicates loop)
    Back,
    /// Forward edge (source dominates target, not tree edge)
    Forward,
    /// Cross edge (neither dominates)
    Cross,
}
