use crate::{
    decode::{self, InstrKind, Operands},
    wasm::WasmModule,
};
use binaryninja::binary_view::{BinaryView, BinaryViewExt};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, RwLock},
};

const ANALYSIS_KEY: &str = "wasm_analysis";

pub struct AnalyzedModules {
    pub modules: HashMap<u64, WasmModuleAnalysis>,
}

impl AnalyzedModules {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn get_for_view(&self, view: &BinaryView) -> Option<&WasmModuleAnalysis> {
        let handle = view.handle as u64;
        self.modules.get(&handle)
    }

    pub fn get_for_view_mut(&mut self, view: &BinaryView) -> Option<&mut WasmModuleAnalysis> {
        let handle = view.handle as u64;
        self.modules.get_mut(&handle)
    }

    pub fn register_for_view(&mut self, view: &BinaryView, analysis: WasmModuleAnalysis) {
        self.modules.insert(view.handle as u64, analysis);
        view.store_metadata(ANALYSIS_KEY, view.handle as u64, true);
    }
}

pub static ANALYZED_MODULES: LazyLock<RwLock<AnalyzedModules>> =
    LazyLock::new(|| RwLock::new(AnalyzedModules::new()));

pub struct WasmModuleAnalysis {
    pub module: Arc<WasmModule>,
    pub functions: HashMap<u64, FunctionAnalysis>,
}

impl WasmModuleAnalysis {
    pub fn new(module: Arc<WasmModule>) -> Self {
        Self {
            module,
            functions: HashMap::new(),
        }
    }

    pub fn analyze_function(&mut self, func_addr: u64, code: &[u8], param_count: usize) {
        let analysis = analyze_function(code, func_addr, param_count, &self.module);
        self.functions.insert(func_addr, analysis);
    }

    pub fn get_function_analysis_for_instruction(
        &self,
        instruction_addr: u64,
    ) -> Option<&FunctionAnalysis> {
        let func_addr = self
            .functions
            .iter()
            .find(|(addr, analysis)| {
                instruction_addr >= **addr && instruction_addr < analysis.end_address
            })
            .map(|(_, analysis)| analysis)?;
        Some(func_addr)
    }

    pub fn get_function_analysis(&self, func_addr: u64) -> Option<&FunctionAnalysis> {
        self.functions.get(&func_addr)
    }
}

pub struct FunctionAnalysis {
    pub start_address: u64,
    pub end_address: u64,
    /// Number of function parameters. In WASM, locals 0..param_count are the parameters.
    pub param_count: usize,
    /// Map of WASM block start address to block end address.
    pub blocks: HashMap<u64, u64>,
    /// Map of branch instruction address to target address
    pub branch_targets: HashMap<u64, u64>,
    /// Map of `if` instruction address to false branch target (else or end address).
    /// When the condition is false, control jumps to this target.
    pub if_targets: HashMap<u64, u64>,
    /// Map of `else` instruction address to end address.
    /// At the end of the true branch, control jumps past the else block to this target.
    pub else_targets: HashMap<u64, u64>,
    /// Map of `br_table` instruction address to resolved targets.
    /// Contains (labels_targets, default_target) where labels_targets[i] is the resolved
    /// address for case i, and default_target is the address for out-of-bounds indices.
    pub br_table_targets: HashMap<u64, (Vec<u64>, u64)>,
    /// Sorted list of addresses where basic blocks begin.
    ///
    /// A basic block is a sequence of instructions with:
    /// - Single entry point (only jumped to from outside)
    /// - Single exit point (ends with branch, return, or falls through)
    ///
    /// New basic blocks start at:
    /// - Function entry
    /// - Branch targets (destinations of br/br_if)
    /// - Instructions following control flow (block/loop/if/else/end/br/br_if/return/unreachable)
    ///
    /// Note: These are CFG basic blocks, not WASM structured blocks.
    /// A single WASM `block..end` may contain multiple basic blocks if it has branches.
    pub basic_block_starts: Vec<u64>,
    /// Map of instruction address to stack depth at that instruction (before execution).
    /// Used for LLIL lifting to determine which temp register represents the stack slot.
    pub instruction_stack_depths: HashMap<u64, u32>,
}

pub fn analyze_function(
    code: &[u8],
    base_addr: u64,
    param_count: usize,
    module: &WasmModule,
) -> FunctionAnalysis {
    use tracing::info;

    let blocks = resolve_block_indicies(code, base_addr);
    let (branch_targets, br_table_targets) = resolve_branch_targets(code, base_addr, &blocks);
    let (if_targets, else_targets) = resolve_if_else_targets(code, base_addr);
    let basic_block_starts = resolve_basic_blocks_with_br_table(
        code,
        base_addr,
        &branch_targets,
        &if_targets,
        &else_targets,
        &br_table_targets,
    );
    let instruction_stack_depths = analyze_stack_depths(code, base_addr, module);

    info!(
        "analyze_function: base={:#x} len={} blocks={:?} targets={:?} if_targets={:?} else_targets={:?} br_table_targets={:?}",
        base_addr,
        code.len(),
        blocks,
        branch_targets,
        if_targets,
        else_targets,
        br_table_targets
    );

    FunctionAnalysis {
        start_address: base_addr,
        end_address: base_addr + code.len() as u64,
        param_count,
        blocks,
        branch_targets,
        if_targets,
        else_targets,
        br_table_targets,
        basic_block_starts,
        instruction_stack_depths,
    }
}

fn resolve_branch_targets(
    code: &[u8],
    base_addr: u64,
    blocks: &HashMap<u64, u64>,
) -> (HashMap<u64, u64>, HashMap<u64, (Vec<u64>, u64)>) {
    let mut branch_targets = HashMap::new();
    let mut br_table_targets = HashMap::new();
    let mut block_stack: Vec<BlockInfo> = Vec::new();
    let mut offset = 0;

    while offset < code.len() {
        let addr = base_addr + offset as u64;
        let Some(instr) = decode::decode(&code[offset..]) else {
            break;
        };

        match instr.kind {
            InstrKind::Block => {
                block_stack.push(BlockInfo {
                    kind: BlockKind::Block,
                    start: addr,
                });
            }
            InstrKind::Loop => {
                block_stack.push(BlockInfo {
                    kind: BlockKind::Loop,
                    start: addr,
                });
            }
            InstrKind::If => {
                block_stack.push(BlockInfo {
                    kind: BlockKind::If,
                    start: addr,
                });
            }
            InstrKind::End => {
                block_stack.pop();
            }
            InstrKind::Branch | InstrKind::CondBranch => {
                if let Operands::Index(depth) = instr.operands {
                    if let Some(target) = resolve_branch(&block_stack, blocks, depth) {
                        branch_targets.insert(addr, target);
                    }
                }
            }
            InstrKind::BrTable => {
                if let Operands::BrTable { labels, default } = &instr.operands {
                    // Resolve all label depths to addresses
                    let label_targets: Vec<u64> = labels
                        .iter()
                        .filter_map(|&depth| resolve_branch(&block_stack, blocks, depth))
                        .collect();

                    // Resolve default depth to address
                    if let Some(default_target) = resolve_branch(&block_stack, blocks, *default) {
                        // Only store if we resolved all targets
                        if label_targets.len() == labels.len() {
                            br_table_targets.insert(addr, (label_targets, default_target));
                        }
                    }
                }
            }
            _ => {}
        }

        offset += instr.len;
    }
    (branch_targets, br_table_targets)
}

#[derive(Clone, Copy)]
enum BlockKind {
    Block,
    Loop,
    If,
}

struct BlockInfo {
    kind: BlockKind,
    start: u64,
}

/// Maps block start addresses to their corresponding end addresses.
fn resolve_block_indicies(code: &[u8], base_addr: u64) -> HashMap<u64, u64> {
    let mut ends = HashMap::new();
    let mut stack = Vec::new();
    let mut offset = 0;

    while offset < code.len() {
        let addr = base_addr + offset as u64;
        let Some(instr) = decode::decode(&code[offset..]) else {
            break;
        };

        match instr.kind {
            InstrKind::Block | InstrKind::Loop | InstrKind::If => {
                stack.push(addr);
            }
            InstrKind::End => {
                if let Some(start) = stack.pop() {
                    ends.insert(start, addr + instr.len as u64);
                }
            }
            _ => {}
        }

        offset += instr.len;
    }

    ends
}

fn resolve_branch(stack: &[BlockInfo], ends: &HashMap<u64, u64>, depth: u32) -> Option<u64> {
    let idx = stack.len().checked_sub(1 + depth as usize)?;
    let block = stack.get(idx)?;

    match block.kind {
        BlockKind::Loop => Some(block.start), // loop: jump to start
        BlockKind::Block | BlockKind::If => ends.get(&block.start).copied(), // block/if: jump to end
    }
}

/// Resolve `if` and `else` control flow targets.
///
/// For `if` instructions:
/// - If there's an `else`, the false branch goes to the instruction after `else`
/// - If there's no `else`, the false branch goes to the instruction after `end`
///
/// For `else` instructions:
/// - At the end of the true branch (just before `else`), we need to jump to after the `end`
/// - The `else_targets` map records where to jump when falling through from the true branch
///
/// Returns (if_targets, else_targets)
fn resolve_if_else_targets(code: &[u8], base_addr: u64) -> (HashMap<u64, u64>, HashMap<u64, u64>) {
    let mut if_targets = HashMap::new();
    let mut else_targets = HashMap::new();

    // Stack of (kind, start_addr, else_info_if_any)
    // For `if` blocks, we track if we've seen an `else` and its instruction length
    struct IfBlockInfo {
        kind: BlockKind,
        start: u64,
        else_info: Option<(u64, usize)>, // (else_addr, else_instr_len)
    }

    let mut stack: Vec<IfBlockInfo> = Vec::new();
    let mut offset = 0;

    while offset < code.len() {
        let addr = base_addr + offset as u64;
        let Some(instr) = decode::decode(&code[offset..]) else {
            break;
        };

        match instr.kind {
            InstrKind::Block => {
                stack.push(IfBlockInfo {
                    kind: BlockKind::Block,
                    start: addr,
                    else_info: None,
                });
            }
            InstrKind::Loop => {
                stack.push(IfBlockInfo {
                    kind: BlockKind::Loop,
                    start: addr,
                    else_info: None,
                });
            }
            InstrKind::If => {
                stack.push(IfBlockInfo {
                    kind: BlockKind::If,
                    start: addr,
                    else_info: None,
                });
            }
            InstrKind::Else => {
                // Record the else address and instruction length for the current if block
                if let Some(block) = stack.last_mut() {
                    if matches!(block.kind, BlockKind::If) {
                        block.else_info = Some((addr, instr.len));
                    }
                }
            }
            InstrKind::End => {
                if let Some(block) = stack.pop() {
                    if matches!(block.kind, BlockKind::If) {
                        let end_addr = addr + instr.len as u64; // Address after the end instruction

                        if let Some((else_addr, else_len)) = block.else_info {
                            // If has an else: false branch goes to instruction after else,
                            // else instruction jumps to after end
                            let else_body_start = else_addr + else_len as u64;
                            if_targets.insert(block.start, else_body_start);
                            else_targets.insert(else_addr, end_addr);
                        } else {
                            // If has no else: false branch goes directly to after end
                            if_targets.insert(block.start, end_addr);
                        }
                    }
                }
            }
            _ => {}
        }

        offset += instr.len;
    }

    (if_targets, else_targets)
}

#[allow(dead_code)]
fn resolve_basic_blocks(
    code: &[u8],
    base_addr: u64,
    branch_targets: &HashMap<u64, u64>,
) -> Vec<u64> {
    resolve_basic_blocks_with_if(code, base_addr, branch_targets, &HashMap::new(), &HashMap::new())
}

fn resolve_basic_blocks_with_if(
    code: &[u8],
    base_addr: u64,
    branch_targets: &HashMap<u64, u64>,
    if_targets: &HashMap<u64, u64>,
    else_targets: &HashMap<u64, u64>,
) -> Vec<u64> {
    resolve_basic_blocks_with_br_table(
        code,
        base_addr,
        branch_targets,
        if_targets,
        else_targets,
        &HashMap::new(),
    )
}

fn resolve_basic_blocks_with_br_table(
    code: &[u8],
    base_addr: u64,
    branch_targets: &HashMap<u64, u64>,
    if_targets: &HashMap<u64, u64>,
    else_targets: &HashMap<u64, u64>,
    br_table_targets: &HashMap<u64, (Vec<u64>, u64)>,
) -> Vec<u64> {
    let mut starts = Vec::new();
    starts.push(base_addr);

    // Add all branch targets as basic block starts
    for &target in branch_targets.values() {
        if !starts.contains(&target) {
            starts.push(target);
        }
    }

    // Add all if false-branch targets as basic block starts
    for &target in if_targets.values() {
        if !starts.contains(&target) {
            starts.push(target);
        }
    }

    // Add all else jump targets as basic block starts
    for &target in else_targets.values() {
        if !starts.contains(&target) {
            starts.push(target);
        }
    }

    // Add all br_table targets as basic block starts
    for (label_targets, default_target) in br_table_targets.values() {
        for &target in label_targets {
            if !starts.contains(&target) {
                starts.push(target);
            }
        }
        if !starts.contains(default_target) {
            starts.push(*default_target);
        }
    }

    let mut offset = 0;
    while offset < code.len() {
        let addr = base_addr + offset as u64;
        let Some(instr) = decode::decode(&code[offset..]) else {
            break;
        };
        let next_addr = addr + instr.len as u64;
        let is_in_function = next_addr < base_addr + code.len() as u64;

        match instr.kind {
            InstrKind::Else => {
                if !starts.contains(&addr) {
                    starts.push(addr);
                }
                if is_in_function && !starts.contains(&next_addr) {
                    starts.push(next_addr);
                }
            }
            InstrKind::Block
            | InstrKind::Loop
            | InstrKind::If
            | InstrKind::Branch
            | InstrKind::CondBranch
            | InstrKind::BrTable
            | InstrKind::End => {
                if is_in_function && !starts.contains(&next_addr) {
                    starts.push(next_addr);
                }
            }
            _ => {}
        }

        offset += instr.len;
    }

    starts.sort();
    starts
}

/// Analyze stack depths at each instruction.
/// Returns a map from instruction address to stack depth (before execution).
/// This is used by LLIL lifting to map stack operations to temp registers.
fn analyze_stack_depths(code: &[u8], base_addr: u64, module: &WasmModule) -> HashMap<u64, u32> {
    let mut depths = HashMap::new();
    let mut depth: u32 = 0;
    let mut offset = 0;

    while offset < code.len() {
        let addr = base_addr + offset as u64;
        let Some(instr) = decode::decode(&code[offset..]) else {
            break;
        };

        // Record depth BEFORE this instruction executes
        depths.insert(addr, depth);

        // Update depth based on instruction effect
        // Stack effects: (pops, pushes)
        let (pops, pushes): (u32, u32) = match instr.kind {
            InstrKind::Const => (0, 1),      // push constant
            InstrKind::LocalGet => (0, 1),   // push local value
            InstrKind::LocalSet => (1, 0),   // pop into local
            InstrKind::LocalTee => (1, 1),   // pop, set local, push back (net 0)
            InstrKind::GlobalGet => (0, 1),  // push global value
            InstrKind::GlobalSet => (1, 0),  // pop into global
            InstrKind::Drop => (1, 0),       // pop and discard
            InstrKind::Select => (3, 1),     // pop cond, val2, val1; push result
            InstrKind::Load => (1, 1),       // pop addr, push value
            InstrKind::Store => (2, 0),      // pop value, pop addr
            InstrKind::MemoryFill => (3, 0), // pop n, val, d
            InstrKind::MemoryCopy => (3, 0), // pop n, s, d
            InstrKind::TableGet => (1, 1),   // pop index, push value
            InstrKind::TableSet => (2, 0),   // pop value, pop index
            InstrKind::BinOp => (2, 1),      // pop 2, push 1
            InstrKind::UnaryOp => (1, 1),    // pop 1, push 1
            InstrKind::Compare => (2, 1),    // pop 2, push i32 result
            InstrKind::Test => (1, 1),       // pop 1, push i32 result
            InstrKind::CondBranch => (1, 0), // br_if pops condition
            InstrKind::BrTable => (1, 0),    // br_table pops index
            InstrKind::Call => {
                // Call: pops param_count args, pushes return_count values
                // Look up function signature from module
                if let Operands::Index(func_idx) = instr.operands {
                    if let Some(func) = module.functions.iter().find(|f| f.index == func_idx) {
                        (func.param_count as u32, func.return_count as u32)
                    } else {
                        // Unknown function - assume no stack effect (conservative)
                        (0, 0)
                    }
                } else if let Operands::Indexes(_type_idx, _) = instr.operands {
                    // call_indirect uses type index - would need type section lookup
                    // For now, assume 2 params and 1 return as a common case
                    // TODO: Look up actual type signature
                    (2, 1)
                } else {
                    (0, 0)
                }
            }
            InstrKind::If => (1, 0), // if pops condition
            // Control flow that doesn't affect stack depth calculation linearly
            InstrKind::Block
            | InstrKind::Loop
            | InstrKind::Else
            | InstrKind::End
            | InstrKind::Branch
            | InstrKind::Return
            | InstrKind::Unreachable
            | InstrKind::Nop
            | InstrKind::Normal => (0, 0),
        };

        depth = depth.saturating_sub(pops);
        depth = depth.saturating_add(pushes);
        offset += instr.len;
    }

    depths
}
