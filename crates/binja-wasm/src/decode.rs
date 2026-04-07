//! WASM instruction decoding using wasmparser

use tracing::error;
use wasmparser::{BinaryReader, Operator};

#[derive(Debug, Clone)]
pub struct Instruction {
    pub len: usize,
    pub name: &'static str,
    pub kind: InstrKind,
    pub operands: Operands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstrKind {
    #[default]
    Normal,
    Block,
    Loop,
    If,
    Else,
    End,
    Branch,
    CondBranch,
    BrTable,
    Return,
    Call,
    Unreachable,
    // Constants
    Const,
    // Local/Global variable access
    LocalGet,
    LocalSet,
    LocalTee,
    GlobalGet,
    GlobalSet,
    // Stack manipulation
    Drop,
    Select,
    // Memory operations
    Load,
    Store,
    // Arithmetic (binary: pop 2, push 1)
    BinOp,
    // Unary (pop 1, push 1)
    UnaryOp,
    // Comparison (pop 2, push 1 i32)
    Compare,
    // Test (pop 1, push 1 i32) - like eqz
    Test,
    // Nop
    Nop,
    // Bulk memory operations
    MemoryFill,
    MemoryCopy,
    // Table operations
    TableGet,
    TableSet,
}

#[derive(Debug, Clone, Default)]
pub enum Operands {
    #[default]
    None,
    BlockType(wasmparser::BlockType),
    Index(u32),
    Indexes(u32, u32),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    MemArg {
        align: u32,
        offset: u64,
    },
    BrTable {
        labels: Vec<u32>,
        default: u32,
    },
}

pub fn decode(data: &[u8]) -> Option<Instruction> {
    let mut reader = BinaryReader::new(data, 0);
    let pos_before = reader.original_position();

    let op = match reader.read_operator() {
        Ok(op) => op,
        Err(err) if err.message() == "unexpected end-of-file" => {
            error!("EOF while decoding instruction at offset {pos_before}: {err:?}");
            return None;
        }
        Err(_) => return None,
    };
    let len = reader.original_position() - pos_before;

    let (name, kind, operands) = decode_operator(op);

    Some(Instruction {
        len,
        name,
        kind,
        operands,
    })
}

fn decode_operator(op: Operator<'_>) -> (&'static str, InstrKind, Operands) {
    use InstrKind::*;
    use Operands::*;

    match op {
        // Control flow
        Operator::Unreachable => ("unreachable", Unreachable, None),
        Operator::Nop => ("nop", Nop, None),
        Operator::Block { blockty } => ("block", Block, BlockType(blockty)),
        Operator::Loop { blockty } => ("loop", Loop, BlockType(blockty)),
        Operator::If { blockty } => ("if", If, BlockType(blockty)),
        Operator::Else => ("else", Else, None),
        Operator::End => ("end", End, None),
        Operator::Br { relative_depth } => ("br", Branch, Index(relative_depth)),
        Operator::BrIf { relative_depth } => ("br_if", CondBranch, Index(relative_depth)),
        Operator::BrTable { targets } => {
            let labels: Vec<u32> = targets.targets().filter_map(|t| t.ok()).collect();
            (
                "br_table",
                BrTable,
                Operands::BrTable {
                    labels,
                    default: targets.default(),
                },
            )
        }
        Operator::Return => ("return", Return, None),
        Operator::Call { function_index } => ("call", Call, Index(function_index)),
        Operator::CallIndirect {
            type_index,
            table_index,
        } => ("call_indirect", Call, Indexes(type_index, table_index)),

        // Parametric
        Operator::Drop => ("drop", Drop, None),
        Operator::Select => ("select", Select, None),
        Operator::TypedSelect { ty: _ } => ("select", Select, None),

        // Variable access
        Operator::LocalGet { local_index } => ("local.get", LocalGet, Index(local_index)),
        Operator::LocalSet { local_index } => ("local.set", LocalSet, Index(local_index)),
        Operator::LocalTee { local_index } => ("local.tee", LocalTee, Index(local_index)),
        Operator::GlobalGet { global_index } => ("global.get", GlobalGet, Index(global_index)),
        Operator::GlobalSet { global_index } => ("global.set", GlobalSet, Index(global_index)),

        // Memory load
        Operator::I32Load { memarg } => (
            "i32.load",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Load { memarg } => (
            "i64.load",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::F32Load { memarg } => (
            "f32.load",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::F64Load { memarg } => (
            "f64.load",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I32Load8S { memarg } => (
            "i32.load8_s",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I32Load8U { memarg } => (
            "i32.load8_u",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I32Load16S { memarg } => (
            "i32.load16_s",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I32Load16U { memarg } => (
            "i32.load16_u",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Load8S { memarg } => (
            "i64.load8_s",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Load8U { memarg } => (
            "i64.load8_u",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Load16S { memarg } => (
            "i64.load16_s",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Load16U { memarg } => (
            "i64.load16_u",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Load32S { memarg } => (
            "i64.load32_s",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Load32U { memarg } => (
            "i64.load32_u",
            Load,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),

        // Memory store
        Operator::I32Store { memarg } => (
            "i32.store",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Store { memarg } => (
            "i64.store",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::F32Store { memarg } => (
            "f32.store",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::F64Store { memarg } => (
            "f64.store",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I32Store8 { memarg } => (
            "i32.store8",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I32Store16 { memarg } => (
            "i32.store16",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Store8 { memarg } => (
            "i64.store8",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Store16 { memarg } => (
            "i64.store16",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),
        Operator::I64Store32 { memarg } => (
            "i64.store32",
            Store,
            MemArg {
                align: memarg.align as u32,
                offset: memarg.offset,
            },
        ),

        // Memory operations
        Operator::MemorySize { mem, .. } => ("memory.size", Normal, Index(mem)),
        Operator::MemoryGrow { mem, .. } => ("memory.grow", Normal, Index(mem)),
        Operator::MemoryCopy { dst_mem, src_mem } => {
            ("memory.copy", MemoryCopy, Indexes(dst_mem, src_mem))
        }
        Operator::MemoryFill { mem } => ("memory.fill", MemoryFill, Index(mem)),

        // Constants
        Operator::I32Const { value } => ("i32.const", Const, I32(value)),
        Operator::I64Const { value } => ("i64.const", Const, I64(value)),
        Operator::F32Const { value } => ("f32.const", Const, F32(f32::from_bits(value.bits()))),
        Operator::F64Const { value } => ("f64.const", Const, F64(f64::from_bits(value.bits()))),

        // i32 comparison
        Operator::I32Eqz => ("i32.eqz", Test, None),
        Operator::I32Eq => ("i32.eq", Compare, None),
        Operator::I32Ne => ("i32.ne", Compare, None),
        Operator::I32LtS => ("i32.lt_s", Compare, None),
        Operator::I32LtU => ("i32.lt_u", Compare, None),
        Operator::I32GtS => ("i32.gt_s", Compare, None),
        Operator::I32GtU => ("i32.gt_u", Compare, None),
        Operator::I32LeS => ("i32.le_s", Compare, None),
        Operator::I32LeU => ("i32.le_u", Compare, None),
        Operator::I32GeS => ("i32.ge_s", Compare, None),
        Operator::I32GeU => ("i32.ge_u", Compare, None),

        // i64 comparison
        Operator::I64Eqz => ("i64.eqz", Test, None),
        Operator::I64Eq => ("i64.eq", Compare, None),
        Operator::I64Ne => ("i64.ne", Compare, None),
        Operator::I64LtS => ("i64.lt_s", Compare, None),
        Operator::I64LtU => ("i64.lt_u", Compare, None),
        Operator::I64GtS => ("i64.gt_s", Compare, None),
        Operator::I64GtU => ("i64.gt_u", Compare, None),
        Operator::I64LeS => ("i64.le_s", Compare, None),
        Operator::I64LeU => ("i64.le_u", Compare, None),
        Operator::I64GeS => ("i64.ge_s", Compare, None),
        Operator::I64GeU => ("i64.ge_u", Compare, None),

        // f32 comparison
        Operator::F32Eq => ("f32.eq", Compare, None),
        Operator::F32Ne => ("f32.ne", Compare, None),
        Operator::F32Lt => ("f32.lt", Compare, None),
        Operator::F32Gt => ("f32.gt", Compare, None),
        Operator::F32Le => ("f32.le", Compare, None),
        Operator::F32Ge => ("f32.ge", Compare, None),

        // f64 comparison
        Operator::F64Eq => ("f64.eq", Compare, None),
        Operator::F64Ne => ("f64.ne", Compare, None),
        Operator::F64Lt => ("f64.lt", Compare, None),
        Operator::F64Gt => ("f64.gt", Compare, None),
        Operator::F64Le => ("f64.le", Compare, None),
        Operator::F64Ge => ("f64.ge", Compare, None),

        // i32 arithmetic
        Operator::I32Clz => ("i32.clz", UnaryOp, None),
        Operator::I32Ctz => ("i32.ctz", UnaryOp, None),
        Operator::I32Popcnt => ("i32.popcnt", UnaryOp, None),
        Operator::I32Add => ("i32.add", BinOp, None),
        Operator::I32Sub => ("i32.sub", BinOp, None),
        Operator::I32Mul => ("i32.mul", BinOp, None),
        Operator::I32DivS => ("i32.div_s", BinOp, None),
        Operator::I32DivU => ("i32.div_u", BinOp, None),
        Operator::I32RemS => ("i32.rem_s", BinOp, None),
        Operator::I32RemU => ("i32.rem_u", BinOp, None),
        Operator::I32And => ("i32.and", BinOp, None),
        Operator::I32Or => ("i32.or", BinOp, None),
        Operator::I32Xor => ("i32.xor", BinOp, None),
        Operator::I32Shl => ("i32.shl", BinOp, None),
        Operator::I32ShrS => ("i32.shr_s", BinOp, None),
        Operator::I32ShrU => ("i32.shr_u", BinOp, None),
        Operator::I32Rotl => ("i32.rotl", BinOp, None),
        Operator::I32Rotr => ("i32.rotr", BinOp, None),

        // i64 arithmetic
        Operator::I64Clz => ("i64.clz", UnaryOp, None),
        Operator::I64Ctz => ("i64.ctz", UnaryOp, None),
        Operator::I64Popcnt => ("i64.popcnt", UnaryOp, None),
        Operator::I64Add => ("i64.add", BinOp, None),
        Operator::I64Sub => ("i64.sub", BinOp, None),
        Operator::I64Mul => ("i64.mul", BinOp, None),
        Operator::I64DivS => ("i64.div_s", BinOp, None),
        Operator::I64DivU => ("i64.div_u", BinOp, None),
        Operator::I64RemS => ("i64.rem_s", BinOp, None),
        Operator::I64RemU => ("i64.rem_u", BinOp, None),
        Operator::I64And => ("i64.and", BinOp, None),
        Operator::I64Or => ("i64.or", BinOp, None),
        Operator::I64Xor => ("i64.xor", BinOp, None),
        Operator::I64Shl => ("i64.shl", BinOp, None),
        Operator::I64ShrS => ("i64.shr_s", BinOp, None),
        Operator::I64ShrU => ("i64.shr_u", BinOp, None),
        Operator::I64Rotl => ("i64.rotl", BinOp, None),
        Operator::I64Rotr => ("i64.rotr", BinOp, None),

        // f32 arithmetic
        Operator::F32Abs => ("f32.abs", UnaryOp, None),
        Operator::F32Neg => ("f32.neg", UnaryOp, None),
        Operator::F32Ceil => ("f32.ceil", UnaryOp, None),
        Operator::F32Floor => ("f32.floor", UnaryOp, None),
        Operator::F32Trunc => ("f32.trunc", UnaryOp, None),
        Operator::F32Nearest => ("f32.nearest", UnaryOp, None),
        Operator::F32Sqrt => ("f32.sqrt", UnaryOp, None),
        Operator::F32Add => ("f32.add", BinOp, None),
        Operator::F32Sub => ("f32.sub", BinOp, None),
        Operator::F32Mul => ("f32.mul", BinOp, None),
        Operator::F32Div => ("f32.div", BinOp, None),
        Operator::F32Min => ("f32.min", BinOp, None),
        Operator::F32Max => ("f32.max", BinOp, None),
        Operator::F32Copysign => ("f32.copysign", BinOp, None),

        // f64 arithmetic
        Operator::F64Abs => ("f64.abs", UnaryOp, None),
        Operator::F64Neg => ("f64.neg", UnaryOp, None),
        Operator::F64Ceil => ("f64.ceil", UnaryOp, None),
        Operator::F64Floor => ("f64.floor", UnaryOp, None),
        Operator::F64Trunc => ("f64.trunc", UnaryOp, None),
        Operator::F64Nearest => ("f64.nearest", UnaryOp, None),
        Operator::F64Sqrt => ("f64.sqrt", UnaryOp, None),
        Operator::F64Add => ("f64.add", BinOp, None),
        Operator::F64Sub => ("f64.sub", BinOp, None),
        Operator::F64Mul => ("f64.mul", BinOp, None),
        Operator::F64Div => ("f64.div", BinOp, None),
        Operator::F64Min => ("f64.min", BinOp, None),
        Operator::F64Max => ("f64.max", BinOp, None),
        Operator::F64Copysign => ("f64.copysign", BinOp, None),

        // Conversions
        Operator::I32WrapI64 => ("i32.wrap_i64", UnaryOp, None),
        Operator::I32TruncF32S => ("i32.trunc_f32_s", UnaryOp, None),
        Operator::I32TruncF32U => ("i32.trunc_f32_u", UnaryOp, None),
        Operator::I32TruncF64S => ("i32.trunc_f64_s", UnaryOp, None),
        Operator::I32TruncF64U => ("i32.trunc_f64_u", UnaryOp, None),
        Operator::I64ExtendI32S => ("i64.extend_i32_s", UnaryOp, None),
        Operator::I64ExtendI32U => ("i64.extend_i32_u", UnaryOp, None),
        Operator::I64TruncF32S => ("i64.trunc_f32_s", UnaryOp, None),
        Operator::I64TruncF32U => ("i64.trunc_f32_u", UnaryOp, None),
        Operator::I64TruncF64S => ("i64.trunc_f64_s", UnaryOp, None),
        Operator::I64TruncF64U => ("i64.trunc_f64_u", UnaryOp, None),
        Operator::F32ConvertI32S => ("f32.convert_i32_s", UnaryOp, None),
        Operator::F32ConvertI32U => ("f32.convert_i32_u", UnaryOp, None),
        Operator::F32ConvertI64S => ("f32.convert_i64_s", UnaryOp, None),
        Operator::F32ConvertI64U => ("f32.convert_i64_u", UnaryOp, None),
        Operator::F32DemoteF64 => ("f32.demote_f64", UnaryOp, None),
        Operator::F64ConvertI32S => ("f64.convert_i32_s", UnaryOp, None),
        Operator::F64ConvertI32U => ("f64.convert_i32_u", UnaryOp, None),
        Operator::F64ConvertI64S => ("f64.convert_i64_s", UnaryOp, None),
        Operator::F64ConvertI64U => ("f64.convert_i64_u", UnaryOp, None),
        Operator::F64PromoteF32 => ("f64.promote_f32", UnaryOp, None),
        Operator::I32ReinterpretF32 => ("i32.reinterpret_f32", UnaryOp, None),
        Operator::I64ReinterpretF64 => ("i64.reinterpret_f64", UnaryOp, None),
        Operator::F32ReinterpretI32 => ("f32.reinterpret_i32", UnaryOp, None),
        Operator::F64ReinterpretI64 => ("f64.reinterpret_i64", UnaryOp, None),

        // Sign extension
        Operator::I32Extend8S => ("i32.extend8_s", UnaryOp, None),
        Operator::I32Extend16S => ("i32.extend16_s", UnaryOp, None),
        Operator::I64Extend8S => ("i64.extend8_s", UnaryOp, None),
        Operator::I64Extend16S => ("i64.extend16_s", UnaryOp, None),
        Operator::I64Extend32S => ("i64.extend32_s", UnaryOp, None),

        // Saturating truncation
        Operator::I32TruncSatF32S => ("i32.trunc_sat_f32_s", UnaryOp, None),
        Operator::I32TruncSatF32U => ("i32.trunc_sat_f32_u", UnaryOp, None),
        Operator::I32TruncSatF64S => ("i32.trunc_sat_f64_s", UnaryOp, None),
        Operator::I32TruncSatF64U => ("i32.trunc_sat_f64_u", UnaryOp, None),
        Operator::I64TruncSatF32S => ("i64.trunc_sat_f32_s", UnaryOp, None),
        Operator::I64TruncSatF32U => ("i64.trunc_sat_f32_u", UnaryOp, None),
        Operator::I64TruncSatF64S => ("i64.trunc_sat_f64_s", UnaryOp, None),
        Operator::I64TruncSatF64U => ("i64.trunc_sat_f64_u", UnaryOp, None),

        // Reference types
        Operator::RefNull { hty: _ } => ("ref.null", Normal, None),
        Operator::RefIsNull => ("ref.is_null", Normal, None),
        Operator::RefFunc { function_index } => ("ref.func", Normal, Index(function_index)),

        // Table operations
        Operator::TableGet { table } => ("table.get", TableGet, Index(table)),
        Operator::TableSet { table } => ("table.set", TableSet, Index(table)),
        Operator::TableGrow { table } => ("table.grow", Normal, Index(table)),
        Operator::TableSize { table } => ("table.size", Normal, Index(table)),
        Operator::TableFill { table } => ("table.fill", Normal, Index(table)),
        Operator::TableCopy {
            dst_table,
            src_table,
        } => ("table.copy", Normal, Indexes(dst_table, src_table)),
        Operator::TableInit { elem_index, table } => {
            ("table.init", Normal, Indexes(elem_index, table))
        }
        Operator::ElemDrop { elem_index } => ("elem.drop", Normal, Index(elem_index)),

        // Data operations
        Operator::MemoryInit { data_index, mem } => {
            ("memory.init", Normal, Indexes(data_index, mem))
        }
        Operator::DataDrop { data_index } => ("data.drop", Normal, Index(data_index)),

        // Catch-all for other instructions (SIMD, atomics, GC, etc.)
        _ => ("unknown", Normal, None),
    }
}
