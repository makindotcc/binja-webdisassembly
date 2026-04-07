use std::borrow::Cow;

use crate::analysis::{ANALYZED_MODULES, FunctionAnalysis};
use crate::decode::{self, InstrKind, Operands};
use crate::view::{GLOBALS_BASE_ADDR, LINEAR_MEMORY_BASE};
use binaryninja::Endianness;
use binaryninja::architecture::{
    Architecture, BasicBlockAnalysisContext, BranchType, CoreArchitecture,
    CustomArchitectureHandle, ImplicitRegisterExtend, InstructionInfo, Register, RegisterId,
    RegisterInfo, UnusedFlag, UnusedIntrinsic, UnusedRegisterStack,
};
use binaryninja::basic_block::PendingBasicBlockEdge;
use binaryninja::binary_view::BinaryViewBase;
use binaryninja::calling_convention::CallingConvention;
use binaryninja::disassembly::{InstructionTextToken, InstructionTextTokenKind};
use binaryninja::function::Function;
use binaryninja::low_level_il::lifting::LowLevelILLabel;
use binaryninja::low_level_il::{
    LowLevelILMutableFunction, LowLevelILRegisterKind, LowLevelILTempRegister,
};
use tracing::{debug, error, warn};

pub struct WasmArchitecture {
    handle: CustomArchitectureHandle<Self>,
    core_arch: CoreArchitecture,
}

impl WasmArchitecture {
    pub fn new(handle: CustomArchitectureHandle<Self>, core_arch: CoreArchitecture) -> Self {
        Self { handle, core_arch }
    }
}

impl AsRef<CoreArchitecture> for WasmArchitecture {
    fn as_ref(&self) -> &CoreArchitecture {
        &self.core_arch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WasmRegister {
    /// Stack pointer (required by Binary Ninja)
    Sp,
    /// Return value register
    Ret,
    /// Argument/local registers
    Arg0,
    Arg1,
    Arg2,
    Arg3,
    Arg4,
    Arg5,
    Arg6,
    Arg7,
}

impl WasmRegister {
    pub const ALL: &'static [WasmRegister] = &[
        WasmRegister::Sp,
        WasmRegister::Ret,
        WasmRegister::Arg0,
        WasmRegister::Arg1,
        WasmRegister::Arg2,
        WasmRegister::Arg3,
        WasmRegister::Arg4,
        WasmRegister::Arg5,
        WasmRegister::Arg6,
        WasmRegister::Arg7,
    ];

    pub const ARGS: &'static [WasmRegister] = &[
        WasmRegister::Arg0,
        WasmRegister::Arg1,
        WasmRegister::Arg2,
        WasmRegister::Arg3,
        WasmRegister::Arg4,
        WasmRegister::Arg5,
        WasmRegister::Arg6,
        WasmRegister::Arg7,
    ];
}

impl Register for WasmRegister {
    type InfoType = Self;

    fn name(&self) -> Cow<'_, str> {
        match self {
            Self::Sp => "sp".into(),
            Self::Ret => "ret".into(),
            Self::Arg0 => "arg0".into(),
            Self::Arg1 => "arg1".into(),
            Self::Arg2 => "arg2".into(),
            Self::Arg3 => "arg3".into(),
            Self::Arg4 => "arg4".into(),
            Self::Arg5 => "arg5".into(),
            Self::Arg6 => "arg6".into(),
            Self::Arg7 => "arg7".into(),
        }
    }

    fn info(&self) -> Self::InfoType {
        *self
    }

    fn id(&self) -> RegisterId {
        match self {
            Self::Sp => 0u32.into(),
            Self::Ret => 1u32.into(),
            Self::Arg0 => 2u32.into(),
            Self::Arg1 => 3u32.into(),
            Self::Arg2 => 4u32.into(),
            Self::Arg3 => 5u32.into(),
            Self::Arg4 => 6u32.into(),
            Self::Arg5 => 7u32.into(),
            Self::Arg6 => 8u32.into(),
            Self::Arg7 => 9u32.into(),
        }
    }
}

impl RegisterInfo for WasmRegister {
    type RegType = Self;

    fn parent(&self) -> Option<Self> {
        None
    }

    fn size(&self) -> usize {
        8 // Match LLIL operation sizes (handles both i32 and i64)
    }

    fn offset(&self) -> usize {
        0
    }

    fn implicit_extend(&self) -> ImplicitRegisterExtend {
        ImplicitRegisterExtend::NoExtend
    }
}

/// Returns the temp register for a WASM local variable.
/// All locals (including parameters) use temp registers to avoid conflicts
/// with calling convention registers during function calls.
fn local_register(raw_idx: u32) -> LowLevelILRegisterKind<WasmRegister> {
    LowLevelILRegisterKind::Temp(LowLevelILTempRegister::new(raw_idx))
}

impl Architecture for WasmArchitecture {
    type Handle = CustomArchitectureHandle<Self>;
    type RegisterInfo = WasmRegister;
    type Register = WasmRegister;
    type RegisterStackInfo = UnusedRegisterStack<WasmRegister>;
    type RegisterStack = UnusedRegisterStack<WasmRegister>;
    type Flag = UnusedFlag;
    type FlagWrite = UnusedFlag;
    type FlagClass = UnusedFlag;
    type FlagGroup = UnusedFlag;
    type Intrinsic = UnusedIntrinsic;

    fn endianness(&self) -> Endianness {
        Endianness::LittleEndian
    }

    fn address_size(&self) -> usize {
        4
    }

    fn default_integer_size(&self) -> usize {
        4
    }

    fn instruction_alignment(&self) -> usize {
        1
    }

    fn max_instr_len(&self) -> usize {
        32 // BrTable may not fit, but binja aligns disassembly view when i set it to bigger value, so TODO
    }

    fn instruction_info(&self, data: &[u8], _addr: u64) -> Option<InstructionInfo> {
        let instr = decode::decode(data)?;
        // Branch info is handled by analyze_basic_blocks
        Some(InstructionInfo::new(instr.len, 0))
    }

    fn analyze_basic_blocks(
        &self,
        function: &mut Function,
        context: &mut BasicBlockAnalysisContext,
    ) {
        let func_start = function.start();
        let view = function.view();

        let analyzed_modules = ANALYZED_MODULES.read().unwrap();
        let Some(analysis) = analyzed_modules.get_for_view(&view) else {
            warn!("analyze_basic_blocks: no module analysis for view");
            context.finalize();
            return;
        };
        let Some(func_analysis) = analysis.get_function_analysis(func_start) else {
            warn!(
                "analyze_basic_blocks: no function analysis for {:#x}",
                func_start
            );
            context.finalize();
            return;
        };
        let func_end = func_analysis.end_address;

        let code_len = (func_end - func_start) as usize;
        let mut code = vec![0u8; code_len];
        let bytes_read = view.read(&mut code, func_start);
        if bytes_read == 0 {
            warn!("analyze_basic_blocks: no code at {func_start:#x}");
            context.finalize();
            return;
        }
        code.truncate(bytes_read);

        debug!(
            "analyze_basic_blocks: func={:#x} end={:#x} len={}",
            func_start,
            func_end,
            code.len()
        );

        let arch = function.arch();
        create_basic_blocks(context, func_analysis, function, &code, arch);
        context.finalize();
        debug!("analyze_basic_blocks: done for {:#x}", func_start);
    }

    fn instruction_llil(
        &self,
        data: &[u8],
        addr: u64,
        il: &LowLevelILMutableFunction,
    ) -> Option<(usize, bool)> {
        let instr = decode::decode(data)?;

        debug!(
            "instruction_llil: addr={:#x} name={} kind={:?} operands={:?}",
            addr, instr.name, instr.kind, instr.operands
        );

        // During linear sweep, function may not exist yet
        let Some(func) = il.function() else {
            return Some((instr.len, false));
        };
        let view = func.view();

        let analyzed_modules = ANALYZED_MODULES.read().unwrap();
        let Some(analysis) = analyzed_modules.get_for_view(&view) else {
            return Some((instr.len, false));
        };
        let Some(func_analysis) = analysis.get_function_analysis_for_instruction(addr) else {
            return Some((instr.len, false));
        };

        // Get the stack depth at this instruction from analysis
        let initial_depth = func_analysis
            .instruction_stack_depths
            .get(&addr)
            .copied()
            .unwrap_or(0);
        let mut stack = StackState::new(initial_depth);

        // At function entry, copy argument registers to local temp registers
        // This separates the calling convention (arg0-arg7) from local storage (temp0-temp7)
        // Use size 8 as safe default (works for both i32 and i64 parameters)
        if addr == func_analysis.start_address {
            for i in 0..func_analysis.param_count.min(WasmRegister::ARGS.len()) {
                let arg_reg = LowLevelILRegisterKind::Arch(WasmRegister::ARGS[i]);
                let local_reg = local_register(i as u32);
                il.set_reg(8, local_reg, il.reg(8, arg_reg)).append();
            }
        }

        match instr.kind {
            InstrKind::Nop => {
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::Block | InstrKind::Loop => {
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::If => {
                // Pop condition from stack
                let cond_reg = stack.pop();

                // Get target for false branch (else or end)
                if let Some(&target) = func_analysis.if_targets.get(&addr) {
                    let fall_through = addr + instr.len as u64;
                    if let (Some(mut true_label), Some(mut false_label)) = (
                        il.label_for_address(fall_through),
                        il.label_for_address(target),
                    ) {
                        let cond = il.reg(8, cond_reg);
                        il.if_expr(cond, &mut true_label, &mut false_label).append();
                        return Some((instr.len, true));
                    }
                }
                // Fallback: emit nop if we can't resolve targets
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::Else => {
                // Jump over else block to end
                if let Some(&target) = func_analysis.else_targets.get(&addr) {
                    if let Some(mut label) = il.label_for_address(target) {
                        il.goto(&mut label).append();
                        return Some((instr.len, true));
                    }
                }
                // Fallback: emit nop if we can't resolve target
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::End => {
                let next_addr = addr + instr.len as u64;
                if next_addr >= func_analysis.end_address {
                    // end at function end = implicit return
                    // Pop the return value from stack and put in ret register
                    // Use size 8 as safe default (works for both i32 and i64 return values)
                    let stack_val = stack.pop();
                    let ret_reg = LowLevelILRegisterKind::Arch(WasmRegister::Ret);
                    il.set_reg(8, ret_reg, il.reg(8, stack_val)).append();
                    // Return - LLIL_RET wants return address, use const 0 as dummy
                    il.ret(il.const_int(8, 0)).append();
                } else {
                    il.nop().append();
                }
                return Some((instr.len, true));
            }
            InstrKind::Branch => {
                // br N - unconditional jump
                if let Some(&target) = func_analysis.branch_targets.get(&addr) {
                    if let Some(mut label) = il.label_for_address(target) {
                        il.goto(&mut label).append();
                    } else {
                        // Label not available yet - emit jump to target as fallback
                        let target_expr = il.const_ptr(target);
                        il.jump(target_expr).append();
                    }
                    return Some((instr.len, true));
                }
                // No branch target - emit nop
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::CondBranch => {
                // br_if N - conditional jump (pops condition)
                // Pop condition from stack regardless of whether we can resolve the branch
                let cond_reg = stack.pop();

                if let Some(&target) = func_analysis.branch_targets.get(&addr) {
                    let fall_through = addr + instr.len as u64;
                    if let (Some(mut true_label), Some(mut false_label)) = (
                        il.label_for_address(target),
                        il.label_for_address(fall_through),
                    ) {
                        let cond = il.reg(8, cond_reg);
                        il.if_expr(cond, &mut true_label, &mut false_label).append();
                        return Some((instr.len, true));
                    }
                    // Labels not available yet - emit jump to target as fallback
                    let target_expr = il.const_ptr(target);
                    il.jump(target_expr).append();
                    return Some((instr.len, true));
                }
                // No branch target found - emit nop as fallback
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::BrTable => {
                // br_table - switch on stack value to multiple targets
                let idx_reg = stack.pop();

                if let Some((label_targets, default_target)) =
                    func_analysis.br_table_targets.get(&addr)
                {
                    // Generate if-else chain for the switch
                    // if (idx == 0) goto target[0]
                    // else if (idx == 1) goto target[1]
                    // ...
                    // else goto default

                    let num_labels = label_targets.len();

                    if num_labels == 0 {
                        // No labels, just jump to default
                        if let Some(mut label) = il.label_for_address(*default_target) {
                            il.goto(&mut label).append();
                        } else {
                            il.jump(il.const_ptr(*default_target)).append();
                        }
                        return Some((instr.len, true));
                    }

                    // Create labels for each case and default
                    for (i, &target) in label_targets.iter().enumerate() {
                        let cmp = il.cmp_e(8, il.reg(8, idx_reg), il.const_int(8, i as u64));

                        if let Some(mut target_label) = il.label_for_address(target) {
                            let mut next_check = LowLevelILLabel::new();
                            il.if_expr(cmp, &mut target_label, &mut next_check).append();
                            il.mark_label(&mut next_check);
                        } else {
                            // Fallback: use indirect jump for unresolved target
                            let mut next_check = LowLevelILLabel::new();
                            let mut jump_label = LowLevelILLabel::new();
                            il.if_expr(cmp, &mut jump_label, &mut next_check).append();
                            il.mark_label(&mut jump_label);
                            il.jump(il.const_ptr(target)).append();
                            il.mark_label(&mut next_check);
                        }
                    }

                    // Default case (index out of bounds)
                    if let Some(mut default_label) = il.label_for_address(*default_target) {
                        il.goto(&mut default_label).append();
                    } else {
                        il.jump(il.const_ptr(*default_target)).append();
                    }

                    return Some((instr.len, true));
                }

                // Fallback: unresolved br_table - emit trap
                il.trap(0).append();
                return Some((instr.len, true));
            }
            InstrKind::Call => {
                // call function_index - need to resolve function address
                if let Operands::Index(func_idx) = instr.operands {
                    let ret_reg = LowLevelILRegisterKind::Arch(WasmRegister::Ret);

                    if let Some(wasm_func) = analysis
                        .module
                        .functions
                        .iter()
                        .find(|f| f.index == func_idx)
                    {
                        let param_count = wasm_func.param_count;
                        let return_count = wasm_func.return_count;

                        // Pop arguments from value stack and set to arg registers
                        // Arguments are popped in reverse order so arg0 gets first arg
                        // Use size 8 as safe default (works for both i32 and i64)
                        for i in (0..param_count.min(8)).rev() {
                            let src = stack.pop();
                            let arg_reg = LowLevelILRegisterKind::Arch(WasmRegister::ARGS[i]);
                            il.set_reg(8, arg_reg, il.reg(8, src)).append();
                        }

                        if wasm_func.code_offset > 0 {
                            let target = il.const_ptr(wasm_func.code_offset as u64);
                            il.call(target).append();
                        } else {
                            // Imported function - mark ret as undefined to show it comes from external
                            il.set_reg(8, ret_reg, il.undefined()).append();
                        }

                        // Push return value(s) onto value stack
                        // Read from ret register - the calling convention tells decompiler
                        // that calls write to this register
                        if return_count > 0 {
                            let dest = stack.push();
                            il.set_reg(8, dest, il.reg(8, ret_reg)).append();
                        }
                        return Some((instr.len, true));
                    }
                } else if let Operands::Indexes(_, _) = instr.operands {
                    // call_indirect - indirect call through table
                    // TODO: handle param/return counts from type index
                    let target_reg = stack.pop();
                    let target = il.reg(8, target_reg);
                    il.call(target).append();
                    // Assume indirect calls return a value
                    let ret_reg = LowLevelILRegisterKind::Arch(WasmRegister::Ret);
                    let dest = stack.push();
                    il.set_reg(8, dest, il.reg(8, ret_reg)).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::Return => {
                // Pop return value from stack and set to ret register
                // Use size 8 as safe default (works for both i32 and i64 return values)
                let stack_val = stack.pop();
                let ret_reg = LowLevelILRegisterKind::Arch(WasmRegister::Ret);
                il.set_reg(8, ret_reg, il.reg(8, stack_val)).append();
                il.ret(il.const_int(8, 0)).append();
                return Some((instr.len, true));
            }
            InstrKind::Unreachable => {
                il.trap(0).append();
                return Some((instr.len, true));
            }
            InstrKind::Const => {
                // Push constant onto stack temp register
                // All stack registers are 8 bytes for consistency
                match instr.operands {
                    Operands::I32(val) => {
                        let dest = stack.push();
                        il.set_reg(8, dest, il.const_int(8, val as u32 as u64))
                            .append();
                    }
                    Operands::I64(val) => {
                        let dest = stack.push();
                        il.set_reg(8, dest, il.const_int(8, val as u64)).append();
                    }
                    Operands::F32(val) => {
                        let dest = stack.push();
                        il.set_reg(8, dest, il.const_int(8, val.to_bits() as u64))
                            .append();
                    }
                    Operands::F64(val) => {
                        let dest = stack.push();
                        il.set_reg(8, dest, il.const_int(8, val.to_bits())).append();
                    }
                    _ => {
                        il.unimplemented().append();
                        return Some((instr.len, false));
                    }
                }
                return Some((instr.len, true));
            }
            InstrKind::LocalGet => {
                // local.get N - push local variable onto stack
                // Use size 8 as safe default (works for both i32 and i64)
                if let Operands::Index(idx) = instr.operands {
                    let local_reg = local_register(idx);
                    let dest = stack.push();
                    il.set_reg(8, dest, il.reg(8, local_reg)).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::LocalSet => {
                // local.set N - pop stack into local variable
                if let Operands::Index(idx) = instr.operands {
                    let local_reg = local_register(idx);
                    let src = stack.pop();
                    il.set_reg(8, local_reg, il.reg(8, src)).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::LocalTee => {
                // local.tee N - set local and keep value on stack (pop, set, push back)
                if let Operands::Index(idx) = instr.operands {
                    let local_reg = local_register(idx);
                    let src = stack.pop();
                    il.set_reg(8, local_reg.clone(), il.reg(8, src)).append();
                    let dest = stack.push();
                    il.set_reg(8, dest, il.reg(8, local_reg)).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::GlobalGet => {
                // global.get N - load global from memory and push onto stack
                if let Operands::Index(idx) = instr.operands {
                    let global_addr = GLOBALS_BASE_ADDR + (idx as u64 * 8);
                    let loaded = il.load(8, il.const_ptr(global_addr));
                    let dest = stack.push();
                    il.set_reg(8, dest, loaded).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::GlobalSet => {
                // global.set N - pop stack and store to global memory
                if let Operands::Index(idx) = instr.operands {
                    let global_addr = GLOBALS_BASE_ADDR + (idx as u64 * 8);
                    let src = stack.pop();
                    il.store(8, il.const_ptr(global_addr), il.reg(8, src))
                        .append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::Drop => {
                // Drop top of stack - just decrement depth, no LLIL needed
                let _ = stack.pop();
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::Select => {
                // select: [val1, val2, cond] -> [cond != 0 ? val1 : val2]
                // cond is always i32, but val1/val2 can be any type - use 8 as safe default
                let cond = stack.pop();
                let val2 = stack.pop();
                let val1 = stack.pop();
                let dest = stack.push();

                let mut true_label = LowLevelILLabel::new();
                let mut false_label = LowLevelILLabel::new();
                let mut end_label = LowLevelILLabel::new();

                // if (cond) goto true_label else goto false_label
                il.if_expr(il.reg(8, cond), &mut true_label, &mut false_label)
                    .append();

                // true_label: dest = val1; goto end
                il.mark_label(&mut true_label);
                il.set_reg(8, dest.clone(), il.reg(8, val1)).append();
                il.goto(&mut end_label).append();

                // false_label: dest = val2
                il.mark_label(&mut false_label);
                il.set_reg(8, dest, il.reg(8, val2)).append();

                // end_label:
                il.mark_label(&mut end_label);

                return Some((instr.len, true));
            }
            InstrKind::Load => {
                // Memory load: pop address, push value
                // Use 8 bytes for everything to avoid LLIL size mismatch warnings
                // Add LINEAR_MEMORY_BASE to map WASM addresses to our virtual segment
                if let Operands::MemArg { offset, .. } = instr.operands {
                    let addr_reg = stack.pop();
                    let base_offset = LINEAR_MEMORY_BASE + offset;
                    let effective_addr =
                        il.add(8, il.reg(8, addr_reg), il.const_int(8, base_offset));
                    let loaded = il.load(8, effective_addr);
                    let dest = stack.push();
                    il.set_reg(8, dest, loaded).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::Store => {
                // Memory store: pop value, pop address
                // Use 8 bytes for everything to avoid LLIL size mismatch warnings
                // Add LINEAR_MEMORY_BASE to map WASM addresses to our virtual segment
                if let Operands::MemArg { offset, .. } = instr.operands {
                    let value_reg = stack.pop();
                    let addr_reg = stack.pop();
                    let base_offset = LINEAR_MEMORY_BASE + offset;
                    let effective_addr =
                        il.add(8, il.reg(8, addr_reg), il.const_int(8, base_offset));
                    il.store(8, effective_addr, il.reg(8, value_reg)).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::BinOp => {
                // Binary operation: pop 2, compute, push 1
                // Use size 8 consistently to match register size and avoid LLIL warnings
                let rhs_reg = stack.pop();
                let lhs_reg = stack.pop();
                let rhs = il.reg(8, rhs_reg);
                let lhs = il.reg(8, lhs_reg);
                let result = match instr.name {
                    "i32.add" | "i64.add" => il.add(8, lhs, rhs),
                    "i32.sub" | "i64.sub" => il.sub(8, lhs, rhs),
                    "i32.mul" | "i64.mul" => il.mul(8, lhs, rhs),
                    "i32.div_s" | "i64.div_s" => il.divs(8, lhs, rhs),
                    "i32.div_u" | "i64.div_u" => il.divu(8, lhs, rhs),
                    "i32.rem_s" | "i64.rem_s" => il.mods(8, lhs, rhs),
                    "i32.rem_u" | "i64.rem_u" => il.modu(8, lhs, rhs),
                    "i32.and" | "i64.and" => il.and(8, lhs, rhs),
                    "i32.or" | "i64.or" => il.or(8, lhs, rhs),
                    "i32.xor" | "i64.xor" => il.xor(8, lhs, rhs),
                    "i32.shl" | "i64.shl" => il.lsl(8, lhs, rhs),
                    "i32.shr_s" | "i64.shr_s" => il.asr(8, lhs, rhs),
                    "i32.shr_u" | "i64.shr_u" => il.lsr(8, lhs, rhs),
                    "i32.rotl" | "i64.rotl" => il.rol(8, lhs, rhs),
                    "i32.rotr" | "i64.rotr" => il.ror(8, lhs, rhs),
                    // Float ops - just use add as placeholder
                    _ => il.add(8, lhs, rhs),
                };
                let dest = stack.push();
                il.set_reg(8, dest, result).append();
                return Some((instr.len, true));
            }
            InstrKind::UnaryOp => {
                // Unary operation: pop 1, compute, push 1
                let val_reg = stack.pop();
                let dest = stack.push();
                match instr.name {
                    "f32.neg" | "f64.neg" => {
                        let result = il.neg(8, il.reg(8, val_reg));
                        il.set_reg(8, dest, result).append();
                    }
                    // No direct LLIL for clz, ctz, popcnt, abs - pass through
                    _ => {
                        il.set_reg(8, dest, il.reg(8, val_reg)).append();
                    }
                };
                return Some((instr.len, true));
            }
            InstrKind::Compare => {
                // Comparison: pop 2, compare, push i32 (0 or 1)
                // Use size 8 consistently to match register size and avoid LLIL warnings
                let rhs_reg = stack.pop();
                let lhs_reg = stack.pop();
                let rhs = il.reg(8, rhs_reg);
                let lhs = il.reg(8, lhs_reg);
                let result = match instr.name {
                    "i32.eq" | "i64.eq" | "f32.eq" | "f64.eq" => il.cmp_e(8, lhs, rhs),
                    "i32.ne" | "i64.ne" | "f32.ne" | "f64.ne" => il.cmp_ne(8, lhs, rhs),
                    "i32.lt_s" | "i64.lt_s" | "f32.lt" | "f64.lt" => il.cmp_slt(8, lhs, rhs),
                    "i32.lt_u" | "i64.lt_u" => il.cmp_ult(8, lhs, rhs),
                    "i32.gt_s" | "i64.gt_s" | "f32.gt" | "f64.gt" => il.cmp_sgt(8, lhs, rhs),
                    "i32.gt_u" | "i64.gt_u" => il.cmp_ugt(8, lhs, rhs),
                    "i32.le_s" | "i64.le_s" | "f32.le" | "f64.le" => il.cmp_sle(8, lhs, rhs),
                    "i32.le_u" | "i64.le_u" => il.cmp_ule(8, lhs, rhs),
                    "i32.ge_s" | "i64.ge_s" | "f32.ge" | "f64.ge" => il.cmp_sge(8, lhs, rhs),
                    "i32.ge_u" | "i64.ge_u" => il.cmp_uge(8, lhs, rhs),
                    _ => il.cmp_e(8, lhs, rhs),
                };
                let dest = stack.push();
                // Result is i32 but store in 8-byte register for consistency
                il.set_reg(8, dest, result).append();
                return Some((instr.len, true));
            }
            InstrKind::Test => {
                // Test: pop 1, test, push i32
                // Use size 8 consistently to match register size and avoid LLIL warnings
                let val_reg = stack.pop();
                let val = il.reg(8, val_reg);
                let result = match instr.name {
                    "i32.eqz" | "i64.eqz" => il.cmp_e(8, val, il.const_int(8, 0)),
                    _ => il.cmp_e(8, val, il.const_int(8, 0)),
                };
                let dest = stack.push();
                // Result is i32 but store in 8-byte register for consistency
                il.set_reg(8, dest, result).append();
                return Some((instr.len, true));
            }
            InstrKind::MemoryFill => {
                // memory.fill: pop n (count), pop val (byte), pop d (dest addr)
                // Equivalent to memset(d, val, n)
                let n_reg = stack.pop();
                let val_reg = stack.pop();
                let d_reg = stack.pop();
                // Call __builtin_memset(dest, val, count) at well-known address
                // Parameters are passed via arg registers
                let arg0 = LowLevelILRegisterKind::Arch(WasmRegister::Arg0);
                let arg1 = LowLevelILRegisterKind::Arch(WasmRegister::Arg1);
                let arg2 = LowLevelILRegisterKind::Arch(WasmRegister::Arg2);
                il.set_reg(4, arg0, il.reg(4, d_reg)).append();
                il.set_reg(4, arg1, il.reg(4, val_reg)).append();
                il.set_reg(4, arg2, il.reg(4, n_reg)).append();
                let target = il.const_ptr(0x80000024); // __builtin_memset address
                il.call(target).append();
                return Some((instr.len, true));
            }
            InstrKind::MemoryCopy => {
                // memory.copy: pop n (count), pop s (src addr), pop d (dest addr)
                // Equivalent to memcpy(d, s, n)
                let n_reg = stack.pop();
                let s_reg = stack.pop();
                let d_reg = stack.pop();
                // Call __builtin_memcpy(dest, src, count) at well-known address
                let arg0 = LowLevelILRegisterKind::Arch(WasmRegister::Arg0);
                let arg1 = LowLevelILRegisterKind::Arch(WasmRegister::Arg1);
                let arg2 = LowLevelILRegisterKind::Arch(WasmRegister::Arg2);
                il.set_reg(4, arg0, il.reg(4, d_reg)).append();
                il.set_reg(4, arg1, il.reg(4, s_reg)).append();
                il.set_reg(4, arg2, il.reg(4, n_reg)).append();
                let target = il.const_ptr(0x80000020); // __builtin_memcpy address
                il.call(target).append();
                return Some((instr.len, true));
            }
            InstrKind::TableGet => {
                // table.get: pop index, push table[index]
                // Tables store function references - treat as opaque values
                let idx_reg = stack.pop();
                let dest = stack.push();
                // Just pass through the index as the "value" for now
                // A more complete implementation would load from a virtual table segment
                il.set_reg(8, dest, il.reg(8, idx_reg)).append();
                return Some((instr.len, true));
            }
            InstrKind::TableSet => {
                // table.set: pop value, pop index - store value at table[index]
                // Tables store function references - treat as opaque values
                let val_reg = stack.pop();
                let _idx_reg = stack.pop();
                // For analysis purposes, emit a nop - the value is stored
                // in the table for later call_indirect use
                il.nop().append();
                // Suppress unused variable warning by using it
                let _ = val_reg;
                return Some((instr.len, true));
            }
            InstrKind::Normal => {
                // Remaining operations
                il.unimplemented().append();
            }
        }

        Some((instr.len, false))
    }

    fn registers_all(&self) -> Vec<Self::Register> {
        WasmRegister::ALL.to_vec()
    }

    fn registers_full_width(&self) -> Vec<Self::Register> {
        WasmRegister::ALL.to_vec()
    }

    fn register_from_id(&self, id: RegisterId) -> Option<Self::Register> {
        match id.0 {
            0 => Some(WasmRegister::Sp),
            1 => Some(WasmRegister::Ret),
            2 => Some(WasmRegister::Arg0),
            3 => Some(WasmRegister::Arg1),
            4 => Some(WasmRegister::Arg2),
            5 => Some(WasmRegister::Arg3),
            6 => Some(WasmRegister::Arg4),
            7 => Some(WasmRegister::Arg5),
            8 => Some(WasmRegister::Arg6),
            9 => Some(WasmRegister::Arg7),
            // Fallback for temp registers - return Sp to provide valid size (4 bytes)
            // Binary Ninja queries register info for temp regs; returning None causes garbage sizes
            _ => Some(WasmRegister::Sp),
        }
    }

    fn stack_pointer_reg(&self) -> Option<Self::Register> {
        Some(WasmRegister::Sp)
    }

    fn handle(&self) -> Self::Handle {
        self.handle
    }

    fn instruction_text(
        &self,
        data: &[u8],
        _addr: u64,
    ) -> Option<(usize, Vec<InstructionTextToken>)> {
        let instr = decode::decode(data)?;
        let mut tokens = Vec::new();

        tokens.push(InstructionTextToken::new(
            instr.name,
            InstructionTextTokenKind::Instruction,
        ));

        // Align operands
        tokens.push(InstructionTextToken::new(
            " ".repeat(20usize.checked_sub(instr.name.len()).unwrap_or(0)),
            InstructionTextTokenKind::Text,
        ));

        match &instr.operands {
            Operands::None => {}
            Operands::BlockType(bt) => {
                use wasmparser::BlockType;
                match bt {
                    BlockType::Empty => {}
                    BlockType::Type(vt) => {
                        tokens.push(InstructionTextToken::new(
                            format!("{:?}", vt).to_lowercase(),
                            InstructionTextTokenKind::Text,
                        ));
                    }
                    BlockType::FuncType(idx) => {
                        tokens.push(InstructionTextToken::new(
                            format!("type {}", idx),
                            InstructionTextTokenKind::Integer {
                                value: *idx as u64,
                                size: None,
                            },
                        ));
                    }
                }
            }
            Operands::Index(idx) => {
                tokens.push(InstructionTextToken::new(
                    format!("{}", idx),
                    InstructionTextTokenKind::Integer {
                        value: *idx as u64,
                        size: None,
                    },
                ));
            }
            Operands::Indexes(a, b) => {
                tokens.push(InstructionTextToken::new(
                    format!("type {}", a),
                    InstructionTextTokenKind::Integer {
                        value: *a as u64,
                        size: None,
                    },
                ));
                if *b != 0 {
                    tokens.push(InstructionTextToken::new(
                        ", ",
                        InstructionTextTokenKind::OperandSeparator,
                    ));
                    tokens.push(InstructionTextToken::new(
                        format!("table {}", b),
                        InstructionTextTokenKind::Integer {
                            value: *b as u64,
                            size: None,
                        },
                    ));
                }
            }
            Operands::I32(val) => {
                tokens.push(InstructionTextToken::new(
                    format!("{:#x}", *val as u32),
                    InstructionTextTokenKind::Integer {
                        value: *val as u32 as u64,
                        size: Some(4),
                    },
                ));
            }
            Operands::I64(val) => {
                tokens.push(InstructionTextToken::new(
                    format!("{:#x}", *val as u64),
                    InstructionTextTokenKind::Integer {
                        value: *val as u64,
                        size: Some(8),
                    },
                ));
            }
            Operands::F32(val) => {
                tokens.push(InstructionTextToken::new(
                    format!("{}", val),
                    InstructionTextTokenKind::FloatingPoint {
                        value: *val as f64,
                        size: Some(4),
                    },
                ));
            }
            Operands::F64(val) => {
                tokens.push(InstructionTextToken::new(
                    format!("{}", val),
                    InstructionTextTokenKind::FloatingPoint {
                        value: *val,
                        size: Some(8),
                    },
                ));
            }
            Operands::MemArg { align, offset } => {
                tokens.push(InstructionTextToken::new(
                    format!("offset={}", offset),
                    InstructionTextTokenKind::Integer {
                        value: *offset,
                        size: None,
                    },
                ));
                tokens.push(InstructionTextToken::new(
                    ", ",
                    InstructionTextTokenKind::OperandSeparator,
                ));
                tokens.push(InstructionTextToken::new(
                    format!("align={}", 1u32 << align),
                    InstructionTextTokenKind::Integer {
                        value: 1u64 << align,
                        size: None,
                    },
                ));
            }
            Operands::BrTable { labels, default } => {
                for (i, label) in labels.iter().enumerate() {
                    if i > 0 {
                        tokens.push(InstructionTextToken::new(
                            ", ",
                            InstructionTextTokenKind::OperandSeparator,
                        ));
                    }
                    tokens.push(InstructionTextToken::new(
                        format!("{}", label),
                        InstructionTextTokenKind::Integer {
                            value: *label as u64,
                            size: None,
                        },
                    ));
                }
                if !labels.is_empty() {
                    tokens.push(InstructionTextToken::new(
                        ", ",
                        InstructionTextTokenKind::OperandSeparator,
                    ));
                }
                tokens.push(InstructionTextToken::new(
                    format!("{}", default),
                    InstructionTextTokenKind::Integer {
                        value: *default as u64,
                        size: None,
                    },
                ));
            }
        }

        Some((instr.len, tokens))
    }
}

/// Analyze basic blocks and determine branch targets.
fn create_basic_blocks(
    context: &mut BasicBlockAnalysisContext,
    func_analysis: &FunctionAnalysis,
    func: &Function,
    code: &[u8],
    arch: CoreArchitecture,
) {
    let func_start = func.start();

    for (block_index, &basic_block_start) in func_analysis.basic_block_starts.iter().enumerate() {
        let basic_block_end = if block_index + 1 < func_analysis.basic_block_starts.len() {
            func_analysis.basic_block_starts[block_index + 1]
        } else {
            func_analysis.end_address
        };
        debug!("  basic block: {basic_block_start:#x}..{basic_block_end:#x}");

        let Some(block) = context.create_basic_block(arch, basic_block_start) else {
            error!("  FAILED to create block at {basic_block_start:#x}");
            continue;
        };
        block.set_end(basic_block_end);

        let mut last_instr_addr = basic_block_start;
        let mut last_instr_kind = InstrKind::Normal;
        let mut offset = (basic_block_start - func_start) as usize;
        while offset < code.len() {
            let addr = func_start + offset as u64;
            if addr >= basic_block_end {
                break;
            }
            let Some(instr) = decode::decode(&code[offset..]) else {
                break;
            };
            last_instr_addr = addr;
            last_instr_kind = instr.kind;
            offset += instr.len;
        }

        match last_instr_kind {
            InstrKind::Branch => {
                if let Some(&target) = func_analysis.branch_targets.get(&last_instr_addr) {
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::UnconditionalBranch,
                        target,
                        arch,
                        fallthrough: false,
                    });
                }
            }
            InstrKind::CondBranch => {
                if let Some(&target) = func_analysis.branch_targets.get(&last_instr_addr) {
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::TrueBranch,
                        target,
                        arch,
                        fallthrough: false,
                    });
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::FalseBranch,
                        target: basic_block_end,
                        arch,
                        fallthrough: true,
                    });
                }
            }
            InstrKind::If => {
                // if instruction: condition true falls through, false goes to else/end
                if let Some(&target) = func_analysis.if_targets.get(&last_instr_addr) {
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::TrueBranch,
                        target: basic_block_end,
                        arch,
                        fallthrough: true,
                    });
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::FalseBranch,
                        target,
                        arch,
                        fallthrough: false,
                    });
                } else {
                    // Fallback: just fall through
                    if basic_block_end < func_analysis.end_address {
                        block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                            branch_type: BranchType::UnconditionalBranch,
                            target: basic_block_end,
                            arch,
                            fallthrough: true,
                        });
                    }
                }
            }
            InstrKind::Else => {
                // else instruction: jump to after end (skip else body from true branch)
                if let Some(&target) = func_analysis.else_targets.get(&last_instr_addr) {
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::UnconditionalBranch,
                        target,
                        arch,
                        fallthrough: false,
                    });
                } else {
                    // Fallback: just fall through
                    if basic_block_end < func_analysis.end_address {
                        block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                            branch_type: BranchType::UnconditionalBranch,
                            target: basic_block_end,
                            arch,
                            fallthrough: true,
                        });
                    }
                }
            }
            InstrKind::Return => {
                block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                    branch_type: BranchType::FunctionReturn,
                    target: 0,
                    arch,
                    fallthrough: false,
                });
            }
            InstrKind::Unreachable => {}
            InstrKind::BrTable => {
                // Add edges to all br_table targets
                if let Some((label_targets, default_target)) =
                    func_analysis.br_table_targets.get(&last_instr_addr)
                {
                    // Add edge to each label target
                    for &target in label_targets {
                        block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                            branch_type: BranchType::UnconditionalBranch,
                            target,
                            arch,
                            fallthrough: false,
                        });
                    }
                    // Add edge to default target
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::UnconditionalBranch,
                        target: *default_target,
                        arch,
                        fallthrough: false,
                    });
                } else {
                    // Fallback: indirect branch if we couldn't resolve targets
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::IndirectBranch,
                        target: 0,
                        arch,
                        fallthrough: false,
                    });
                }
            }
            InstrKind::End => {
                if basic_block_end >= func_analysis.end_address {
                    // end at function end = implicit return
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::FunctionReturn,
                        target: 0,
                        arch,
                        fallthrough: false,
                    });
                } else {
                    // end in middle of function = fall-through
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::UnconditionalBranch,
                        target: basic_block_end,
                        arch,
                        fallthrough: true,
                    });
                }
            }
            _ => {
                if basic_block_end < func_analysis.end_address {
                    block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                        branch_type: BranchType::UnconditionalBranch,
                        target: basic_block_end,
                        arch,
                        fallthrough: true,
                    });
                }
            }
        }

        context.add_basic_block(block);
        debug!(
            "  added block {:#x}..{:#x}",
            basic_block_start, basic_block_end
        );
    }
    debug!("create_basic_blocks: all blocks added");
}

/// WASM calling convention
/// - Arguments passed in temp registers (mapped from WASM locals)
/// - Return value in temp0xFFFE register
pub struct WasmCallingConvention;

impl CallingConvention for WasmCallingConvention {
    fn caller_saved_registers(&self) -> Vec<RegisterId> {
        vec![]
    }

    fn callee_saved_registers(&self) -> Vec<RegisterId> {
        vec![]
    }

    fn int_arg_registers(&self) -> Vec<RegisterId> {
        // Real architecture registers for arguments
        WasmRegister::ARGS.iter().map(|r| r.id()).collect()
    }

    fn float_arg_registers(&self) -> Vec<RegisterId> {
        vec![]
    }

    fn arg_registers_shared_index(&self) -> bool {
        true // int and float args share the same registers
    }

    fn reserved_stack_space_for_arg_registers(&self) -> bool {
        false
    }

    fn stack_adjusted_on_return(&self) -> bool {
        false
    }

    fn is_eligible_for_heuristics(&self) -> bool {
        false
    }

    fn return_int_reg(&self) -> Option<RegisterId> {
        Some(WasmRegister::Ret.id())
    }

    fn return_hi_int_reg(&self) -> Option<RegisterId> {
        None
    }

    fn return_float_reg(&self) -> Option<RegisterId> {
        Some(WasmRegister::Ret.id())
    }

    fn global_pointer_reg(&self) -> Option<RegisterId> {
        None
    }

    fn implicitly_defined_registers(&self) -> Vec<RegisterId> {
        // Ret is implicitly defined by every call instruction
        vec![WasmRegister::Ret.id()]
    }

    fn are_argument_registers_used_for_var_args(&self) -> bool {
        true
    }
}

/// Base offset for stack temp registers.
/// We use temps starting at 0x8000_0000 to avoid conflict with:
/// - Locals (can be any u32 index in WASM)
/// - Return register (0xFFFE)
///
/// Mainstream WASM engines are setting up hard limit on locals to 50k:
/// V8 source code:
/// ```cpp
/// constexpr size_t kV8MaxWasmFunctionLocals = 50000;
/// ```
const STACK_TEMP_BASE: u32 = 0xFFFF;

/// Tracks the WASM value stack depth during LLIL lifting.
/// Each stack slot is represented by a temp register: temp(STACK_TEMP_BASE + depth).
struct StackState {
    depth: u32,
}

impl StackState {
    fn new(initial_depth: u32) -> Self {
        Self {
            depth: initial_depth,
        }
    }

    /// Push a value onto the stack, returning the temp register for the new slot.
    fn push(&mut self) -> LowLevelILTempRegister {
        let temp = LowLevelILTempRegister::new(STACK_TEMP_BASE + self.depth);
        self.depth += 1;
        temp
    }

    /// Pop a value from the stack, returning the temp register for the popped slot.
    fn pop(&mut self) -> LowLevelILTempRegister {
        self.depth = self.depth.saturating_sub(1);
        LowLevelILTempRegister::new(STACK_TEMP_BASE + self.depth)
    }
}
