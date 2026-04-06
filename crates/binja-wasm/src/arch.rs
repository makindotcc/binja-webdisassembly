use std::borrow::Cow;
use std::collections::HashMap;

use crate::analysis::{ANALYZED_MODULES, FunctionAnalysis};
use crate::decode::{self, InstrKind, Operands};
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
use binaryninja::low_level_il::{LowLevelILMutableFunction, LowLevelILTempRegister};
use tracing::{debug, info, warn};

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
    /// Stack pointer (not really used in WASM but required by Binary Ninja)
    Sp,
    /// Return value register
    Ret,
    /// Argument registers (WASM locals 0-7)
    Arg0,
    Arg1,
    Arg2,
    Arg3,
    Arg4,
    Arg5,
    Arg6,
    Arg7,
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
        4
    }

    fn offset(&self) -> usize {
        0
    }

    fn implicit_extend(&self) -> ImplicitRegisterExtend {
        ImplicitRegisterExtend::NoExtend
    }
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
        16
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
        info!("analyze_basic_blocks: done for {:#x}", func_start);
    }

    fn instruction_llil(
        &self,
        data: &[u8],
        addr: u64,
        il: &LowLevelILMutableFunction,
    ) -> Option<(usize, bool)> {
        let instr = decode::decode(data)?;

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

        match instr.kind {
            InstrKind::Nop => {
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::Block | InstrKind::Loop | InstrKind::If | InstrKind::Else => {
                // Structural markers - just emit nop
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::End => {
                let next_addr = addr + instr.len as u64;
                if next_addr >= func_analysis.end_address {
                    // end at function end = implicit return
                    // Use temp register for return value (temp regs work with LLIL API)
                    // Temp 0xFFFE = our "ret" register equivalent
                    let ret_temp = LowLevelILTempRegister::new(0xFFFE);
                    il.set_reg(4, ret_temp, il.pop(4)).append();
                    // Return - LLIL_RET wants return address, use const 0 as dummy
                    il.ret(il.const_int(4, 0)).append();
                } else {
                    // end in middle = fall-through
                    il.nop().append();
                }
                return Some((instr.len, true));
            }
            InstrKind::Branch => {
                // br N - unconditional jump
                if let Some(&target) = func_analysis.branch_targets.get(&addr) {
                    if let Some(mut label) = il.label_for_address(target) {
                        il.goto(&mut label).append();
                        return Some((instr.len, true));
                    }
                }
                il.unimplemented().append();
            }
            InstrKind::CondBranch => {
                // br_if N - conditional jump
                if let Some(&target) = func_analysis.branch_targets.get(&addr) {
                    let fall_through = addr + instr.len as u64;
                    let has_true = il.label_for_address(target).is_some();
                    let has_false = il.label_for_address(fall_through).is_some();
                    if let (Some(mut true_label), Some(mut false_label)) = (
                        il.label_for_address(target),
                        il.label_for_address(fall_through),
                    ) {
                        // Condition is top of stack
                        let cond = il.pop(4);
                        il.if_expr(cond, &mut true_label, &mut false_label).append();
                        return Some((instr.len, true));
                    }
                    info!(
                        "Labels not found: addr={:#x} target={:#x} fall={:#x} has_true={} has_false={}",
                        addr, target, fall_through, has_true, has_false
                    );
                } else {
                    info!("Branch target not in analysis for {:#x}", addr);
                }
                il.unimplemented().append();
            }
            InstrKind::BrTable => {
                // br_table - indirect jump based on stack value
                let index = il.pop(4);
                il.jump(index).append();
                return Some((instr.len, true));
            }
            InstrKind::Call => {
                // call function_index - need to resolve function address
                if let Operands::Index(func_idx) = instr.operands {
                    if let Some(wasm_func) = analysis.module.functions.get(func_idx as usize) {
                        if wasm_func.code_offset > 0 {
                            let target = il.const_ptr(wasm_func.code_offset as u64);
                            il.call(target).append();
                            return Some((instr.len, true));
                        }
                    }
                } else if let Operands::Indexes(_, _) = instr.operands {
                    // call_indirect - indirect call through table
                    let target = il.pop(4);
                    il.call(target).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::Return => {
                il.ret(il.const_int(4, 0)).append();
                return Some((instr.len, true));
            }
            InstrKind::Unreachable => {
                il.trap(0).append();
                return Some((instr.len, true));
            }
            InstrKind::Const => {
                // Push constant onto stack
                match instr.operands {
                    Operands::I32(val) => {
                        il.push(4, il.const_int(4, val as u32 as u64)).append();
                    }
                    Operands::I64(val) => {
                        il.push(8, il.const_int(8, val as u64)).append();
                    }
                    Operands::F32(val) => {
                        il.push(4, il.const_int(4, val.to_bits() as u64)).append();
                    }
                    Operands::F64(val) => {
                        il.push(8, il.const_int(8, val.to_bits())).append();
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
                // Use temp register to represent local variable
                if let Operands::Index(idx) = instr.operands {
                    let temp = LowLevelILTempRegister::new(idx);
                    let val = il.reg(4, temp);
                    il.push(4, val).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::LocalSet => {
                // local.set N - pop stack into local variable
                if let Operands::Index(idx) = instr.operands {
                    let val = il.pop(4);
                    let temp = LowLevelILTempRegister::new(idx);
                    il.set_reg(4, temp, val).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::LocalTee => {
                // local.tee N - set local and keep value on stack
                if let Operands::Index(idx) = instr.operands {
                    // Pop, set, push back (equivalent to tee)
                    let val = il.pop(4);
                    let temp = LowLevelILTempRegister::new(idx);
                    il.set_reg(4, temp, val).append();
                    let val2 = il.reg(4, LowLevelILTempRegister::new(idx));
                    il.push(4, val2).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::GlobalGet | InstrKind::GlobalSet => {
                // Global access - treat as memory operation
                il.unimplemented().append();
            }
            InstrKind::Drop => {
                // Drop top of stack
                il.pop(4).build();
                il.nop().append();
                return Some((instr.len, true));
            }
            InstrKind::Select => {
                // select: [val1, val2, cond] -> [val1 if cond else val2]
                let _cond = il.pop(4);
                let _val2 = il.pop(4);
                let _val1 = il.pop(4);
                // Would need if expression but simplified:
                il.push(4, il.const_int(4, 0)).append(); // placeholder
                return Some((instr.len, true));
            }
            InstrKind::Load => {
                // Memory load: pop address, push value
                if let Operands::MemArg { offset, .. } = instr.operands {
                    let addr_val = il.pop(4);
                    let effective_addr = if offset > 0 {
                        il.add(4, addr_val, il.const_int(4, offset))
                    } else {
                        addr_val
                    };
                    let size = if instr.name.contains("64") { 8 } else { 4 };
                    let loaded = il.load(size, effective_addr);
                    il.push(size, loaded).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::Store => {
                // Memory store: pop value, pop address
                if let Operands::MemArg { offset, .. } = instr.operands {
                    let size = if instr.name.contains("64") { 8 } else { 4 };
                    let value = il.pop(size);
                    let addr_val = il.pop(4);
                    let effective_addr = if offset > 0 {
                        il.add(4, addr_val, il.const_int(4, offset))
                    } else {
                        addr_val
                    };
                    il.store(size, effective_addr, value).append();
                    return Some((instr.len, true));
                }
                il.unimplemented().append();
            }
            InstrKind::BinOp => {
                // Binary operation: pop 2, compute, push 1
                let size = if instr.name.contains("64") { 8 } else { 4 };
                let rhs = il.pop(size);
                let lhs = il.pop(size);
                let result = match instr.name {
                    "i32.add" | "i64.add" => il.add(size, lhs, rhs),
                    "i32.sub" | "i64.sub" => il.sub(size, lhs, rhs),
                    "i32.mul" | "i64.mul" => il.mul(size, lhs, rhs),
                    "i32.div_s" | "i64.div_s" => il.divs(size, lhs, rhs),
                    "i32.div_u" | "i64.div_u" => il.divu(size, lhs, rhs),
                    "i32.rem_s" | "i64.rem_s" => il.mods(size, lhs, rhs),
                    "i32.rem_u" | "i64.rem_u" => il.modu(size, lhs, rhs),
                    "i32.and" | "i64.and" => il.and(size, lhs, rhs),
                    "i32.or" | "i64.or" => il.or(size, lhs, rhs),
                    "i32.xor" | "i64.xor" => il.xor(size, lhs, rhs),
                    "i32.shl" | "i64.shl" => il.lsl(size, lhs, rhs),
                    "i32.shr_s" | "i64.shr_s" => il.asr(size, lhs, rhs),
                    "i32.shr_u" | "i64.shr_u" => il.lsr(size, lhs, rhs),
                    "i32.rotl" | "i64.rotl" => il.rol(size, lhs, rhs),
                    "i32.rotr" | "i64.rotr" => il.ror(size, lhs, rhs),
                    // Float ops - just use add as placeholder
                    _ => il.add(size, lhs, rhs),
                };
                il.push(size, result).append();
                return Some((instr.len, true));
            }
            InstrKind::UnaryOp => {
                // Unary operation: pop 1, compute, push 1
                let size = if instr.name.contains("64") { 8 } else { 4 };
                let val = il.pop(size);
                let result = match instr.name {
                    "i32.clz" | "i64.clz" => val,       // No direct LLIL for clz
                    "i32.ctz" | "i64.ctz" => val,       // No direct LLIL for ctz
                    "i32.popcnt" | "i64.popcnt" => val, // No direct LLIL for popcnt
                    "f32.neg" | "f64.neg" => il.neg(size, val),
                    "f32.abs" | "f64.abs" => val, // No direct abs
                    _ => val,                     // Pass through for conversions etc
                };
                il.push(size, result).append();
                return Some((instr.len, true));
            }
            InstrKind::Compare => {
                // Comparison: pop 2, compare, push i32 (0 or 1)
                let size = if instr.name.contains("64") && !instr.name.starts_with("f") {
                    8
                } else {
                    4
                };
                let rhs = il.pop(size);
                let lhs = il.pop(size);
                let result = match instr.name {
                    "i32.eq" | "i64.eq" | "f32.eq" | "f64.eq" => il.cmp_e(size, lhs, rhs),
                    "i32.ne" | "i64.ne" | "f32.ne" | "f64.ne" => il.cmp_ne(size, lhs, rhs),
                    "i32.lt_s" | "i64.lt_s" | "f32.lt" | "f64.lt" => il.cmp_slt(size, lhs, rhs),
                    "i32.lt_u" | "i64.lt_u" => il.cmp_ult(size, lhs, rhs),
                    "i32.gt_s" | "i64.gt_s" | "f32.gt" | "f64.gt" => il.cmp_sgt(size, lhs, rhs),
                    "i32.gt_u" | "i64.gt_u" => il.cmp_ugt(size, lhs, rhs),
                    "i32.le_s" | "i64.le_s" | "f32.le" | "f64.le" => il.cmp_sle(size, lhs, rhs),
                    "i32.le_u" | "i64.le_u" => il.cmp_ule(size, lhs, rhs),
                    "i32.ge_s" | "i64.ge_s" | "f32.ge" | "f64.ge" => il.cmp_sge(size, lhs, rhs),
                    "i32.ge_u" | "i64.ge_u" => il.cmp_uge(size, lhs, rhs),
                    _ => il.cmp_e(size, lhs, rhs),
                };
                il.push(4, result).append(); // Result is always i32
                return Some((instr.len, true));
            }
            InstrKind::Test => {
                // Test: pop 1, test, push i32
                let size = if instr.name.contains("64") { 8 } else { 4 };
                let val = il.pop(size);
                let result = match instr.name {
                    "i32.eqz" | "i64.eqz" => il.cmp_e(size, val, il.const_int(size, 0)),
                    _ => il.cmp_e(size, val, il.const_int(size, 0)),
                };
                il.push(4, result).append();
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
        vec![
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
        ]
    }

    fn registers_full_width(&self) -> Vec<Self::Register> {
        vec![
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
        ]
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
            _ => None,
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
        info!("  basic block: {basic_block_start:#x}..{basic_block_end:#x}");

        let Some(block) = context.create_basic_block(arch, basic_block_start) else {
            info!("  FAILED to create block at {basic_block_start:#x}");
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
                block.add_pending_outgoing_edge(&PendingBasicBlockEdge {
                    branch_type: BranchType::IndirectBranch,
                    target: 0,
                    arch,
                    fallthrough: false,
                });
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
        info!(
            "  added block {:#x}..{:#x}",
            basic_block_start, basic_block_end
        );
    }
}

/// WASM calling convention
/// - Arguments passed in temp registers (mapped from WASM locals)
/// - Return value in temp0xFFFE register
pub struct WasmCallingConvention;

/// Temp register ID used for return value in LLIL
/// Must match what we use in instruction_llil for End instruction
const RET_TEMP_ID: u32 = 0xFFFE;

impl CallingConvention for WasmCallingConvention {
    fn caller_saved_registers(&self) -> Vec<RegisterId> {
        // All temp registers are caller-saved in WASM
        vec![
            RegisterId(RET_TEMP_ID | 0x8000_0000), // return register
            RegisterId(0 | 0x8000_0000), // temp0
            RegisterId(1 | 0x8000_0000), // temp1
            RegisterId(2 | 0x8000_0000), // temp2
            RegisterId(3 | 0x8000_0000), // temp3
            RegisterId(4 | 0x8000_0000), // temp4
            RegisterId(5 | 0x8000_0000), // temp5
            RegisterId(6 | 0x8000_0000), // temp6
            RegisterId(7 | 0x8000_0000), // temp7
        ]
    }

    fn callee_saved_registers(&self) -> Vec<RegisterId> {
        vec![]
    }

    fn int_arg_registers(&self) -> Vec<RegisterId> {
        // WASM parameters are locals 0, 1, 2, ... which we lift as temp0, temp1, temp2, ...
        // Temp register IDs have high bit set: idx | 0x8000_0000
        vec![
            RegisterId(0 | 0x8000_0000), // temp0 = arg0
            RegisterId(1 | 0x8000_0000), // temp1 = arg1
            RegisterId(2 | 0x8000_0000), // temp2 = arg2
            RegisterId(3 | 0x8000_0000), // temp3 = arg3
            RegisterId(4 | 0x8000_0000), // temp4 = arg4
            RegisterId(5 | 0x8000_0000), // temp5 = arg5
            RegisterId(6 | 0x8000_0000), // temp6 = arg6
            RegisterId(7 | 0x8000_0000), // temp7 = arg7
        ]
    }

    fn float_arg_registers(&self) -> Vec<RegisterId> {
        // Use same temp registers for float args
        vec![
            RegisterId(0 | 0x8000_0000),
            RegisterId(1 | 0x8000_0000),
            RegisterId(2 | 0x8000_0000),
            RegisterId(3 | 0x8000_0000),
        ]
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
        // Use temp register with high bit set (same as LowLevelILTempRegister::new(RET_TEMP_ID).id())
        Some(RegisterId(RET_TEMP_ID | 0x8000_0000))
    }

    fn return_hi_int_reg(&self) -> Option<RegisterId> {
        None
    }

    fn return_float_reg(&self) -> Option<RegisterId> {
        // Use same temp register for float returns
        Some(RegisterId(RET_TEMP_ID | 0x8000_0000))
    }

    fn global_pointer_reg(&self) -> Option<RegisterId> {
        None
    }

    fn implicitly_defined_registers(&self) -> Vec<RegisterId> {
        vec![]
    }

    fn are_argument_registers_used_for_var_args(&self) -> bool {
        true
    }
}
