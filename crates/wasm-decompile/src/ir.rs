//! Intermediate Representation for WASM decompilation
//!
//! This IR is designed to be transformed by multiple passes,
//! with type annotations that get refined as analysis progresses.

use std::collections::HashMap;

/// A WASM module in IR form
#[derive(Debug, Clone)]
pub struct Module {
    /// Functions in the module
    pub functions: Vec<Function>,
    /// Global variables
    pub globals: Vec<Global>,
    /// Memory sections (initial data)
    pub memory: Vec<u8>,
    /// Initial memory size in pages
    pub memory_pages: u32,
    /// Data segments with their offsets
    pub data_segments: Vec<DataSegment>,
    /// Imported functions
    pub imports: Vec<Import>,
    /// Exported names
    pub exports: HashMap<u32, String>,
    /// Function type signatures
    pub types: Vec<FuncType>,
    /// Function index -> type index mapping
    pub func_types: Vec<u32>,
    /// Element segments (function tables)
    pub elements: Vec<ElementSegment>,
    /// Table size (from table section)
    pub table_size: u32,
}

/// An element segment (populates a function table)
#[derive(Debug, Clone)]
pub struct ElementSegment {
    /// Offset into the table
    pub offset: u32,
    /// Function indices in this segment
    pub func_indices: Vec<u32>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            globals: Vec::new(),
            memory: Vec::new(),
            memory_pages: 0,
            data_segments: Vec::new(),
            imports: Vec::new(),
            exports: HashMap::new(),
            types: Vec::new(),
            func_types: Vec::new(),
            elements: Vec::new(),
            table_size: 0,
        }
    }

    /// Get a string from memory at given offset and length
    pub fn get_string(&self, offset: usize, len: usize) -> Option<String> {
        if offset + len <= self.memory.len() {
            String::from_utf8(self.memory[offset..offset + len].to_vec()).ok()
        } else {
            None
        }
    }

    /// Get a null-terminated string from memory
    pub fn get_cstring(&self, offset: usize) -> Option<String> {
        if offset >= self.memory.len() {
            return None;
        }
        let end = self.memory[offset..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| offset + p)?;
        String::from_utf8(self.memory[offset..end].to_vec()).ok()
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

/// Data segment with offset
#[derive(Debug, Clone)]
pub struct DataSegment {
    pub offset: u32,
    pub data: Vec<u8>,
}

/// Import entry
#[derive(Debug, Clone)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Function(u32), // type index
    Global(ValType),
    Memory,
    Table,
}

/// Function type signature
#[derive(Debug, Clone)]
pub struct FuncType {
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
}

/// Value types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
}

/// Global variable
#[derive(Debug, Clone)]
pub struct Global {
    pub ty: ValType,
    pub mutable: bool,
    pub init: Expr,
}

/// A function in IR form
#[derive(Debug, Clone)]
pub struct Function {
    /// Function index
    pub index: u32,
    /// Optional name (from export or debug info)
    pub name: Option<String>,
    /// Parameter types
    pub params: Vec<ValType>,
    /// Return types
    pub results: Vec<ValType>,
    /// Local variable types (excluding params)
    pub locals: Vec<ValType>,
    /// Function body as a block
    pub body: Block,
    /// Whether this is an imported function
    pub is_import: bool,
}

/// A block of statements
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

impl Block {
    pub fn new() -> Self {
        Self { stmts: Vec::new() }
    }

    pub fn with_stmts(stmts: Vec<Stmt>) -> Self {
        Self { stmts }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

/// Statement types
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Local variable assignment
    LocalSet { local: u32, value: Expr },
    /// Global variable assignment
    GlobalSet { global: u32, value: Expr },
    /// Memory store
    Store {
        addr: Expr,
        offset: u32,
        value: Expr,
        size: MemSize,
    },
    /// Expression statement (for side effects like calls)
    Expr(Expr),
    /// Return statement
    Return(Option<Expr>),
    /// If statement
    If {
        cond: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },
    /// Block (for structured control flow)
    Block {
        label: u32,
        body: Block,
    },
    /// Loop
    Loop {
        label: u32,
        body: Block,
    },
    /// Branch
    Br { label: u32, is_loop: bool },
    /// Conditional branch
    BrIf { label: u32, cond: Expr, is_loop: bool },
    /// Branch table
    BrTable {
        index: Expr,
        targets: Vec<BranchTarget>,
        default: BranchTarget,
    },
    /// Do-while loop (recovered from WASM loop pattern)
    DoWhile { body: Block, cond: Expr },
    /// While loop (recovered from WASM block+loop pattern)
    While { cond: Expr, body: Block },
    /// Switch statement (recovered from br_table + nested blocks)
    Switch {
        index: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Block>,
    },
    /// Try/finally (for epilog cleanup like stack pointer restore)
    TryFinally {
        body: Block,
        finally_block: Block,
    },
    /// Unreachable
    Unreachable,
    /// No-op (placeholder)
    Nop,
    /// Drop value
    Drop(Expr),
}

/// A case in a switch statement
#[derive(Debug, Clone)]
pub struct SwitchCase {
    /// Which index values map to this case
    pub values: Vec<u32>,
    /// The case body
    pub body: Block,
}

/// Branch target for br_table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchTarget {
    pub label: u32,
    pub is_loop: bool,
}

/// Memory operation size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemSize {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

/// Expression with type annotation
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: InferredType,
}

impl Expr {
    pub fn new(kind: ExprKind) -> Self {
        Self {
            kind,
            ty: InferredType::Unknown,
        }
    }

    pub fn with_type(kind: ExprKind, ty: InferredType) -> Self {
        Self { kind, ty }
    }

    pub fn i32_const(val: i32) -> Self {
        Self::with_type(ExprKind::I32Const(val), InferredType::I32)
    }

    pub fn i64_const(val: i64) -> Self {
        Self::with_type(ExprKind::I64Const(val), InferredType::I64)
    }

    pub fn f32_const(val: f32) -> Self {
        Self::with_type(ExprKind::F32Const(val), InferredType::F32)
    }

    pub fn f64_const(val: f64) -> Self {
        Self::with_type(ExprKind::F64Const(val), InferredType::F64)
    }

    pub fn local(idx: u32) -> Self {
        Self::new(ExprKind::Local(idx))
    }

    pub fn global(idx: u32) -> Self {
        Self::new(ExprKind::Global(idx))
    }
}

/// Expression kinds
#[derive(Debug, Clone)]
pub enum ExprKind {
    // Constants
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),

    // Variables
    Local(u32),
    Global(u32),

    // Binary operations
    BinOp(BinOp, Box<Expr>, Box<Expr>),

    // Unary operations
    UnaryOp(UnaryOp, Box<Expr>),

    // Comparison (op, left, right, operand_type)
    Compare(CmpOp, Box<Expr>, Box<Expr>, InferredType),

    // Memory load
    Load {
        addr: Box<Expr>,
        offset: u32,
        size: MemSize,
        signed: bool,
    },

    // Function call
    Call {
        func: u32,
        args: Vec<Expr>,
    },

    // Indirect call
    CallIndirect {
        type_idx: u32,
        table_idx: u32,
        index: Box<Expr>,
        args: Vec<Expr>,
    },

    // Select (ternary)
    Select {
        cond: Box<Expr>,
        then_val: Box<Expr>,
        else_val: Box<Expr>,
    },

    // Type conversions
    Convert {
        op: ConvertOp,
        expr: Box<Expr>,
    },

    // --- High-level expressions (added by passes) ---
    /// Resolved string literal from memory
    StringLiteral(String),

    /// Go string (ptr, len) pair
    GoString {
        ptr: Box<Expr>,
        len: Box<Expr>,
    },

    /// Go slice (ptr, len, cap)
    GoSlice {
        ptr: Box<Expr>,
        len: Box<Expr>,
        cap: Box<Expr>,
    },

    /// Go interface (type, data)
    GoInterface {
        type_ptr: Box<Expr>,
        data: Box<Expr>,
    },

    /// Memory address that was resolved to point to a string
    ResolvedPointer {
        addr: u32,
        resolved: String,
    },
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Integer
    Add,
    Sub,
    Mul,
    DivS,
    DivU,
    RemS,
    RemU,
    And,
    Or,
    Xor,
    Shl,
    ShrS,
    ShrU,
    Rotl,
    Rotr,

    // Float
    FAdd,
    FSub,
    FMul,
    FDiv,
    FMin,
    FMax,
    FCopysign,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    // Integer
    Clz,
    Ctz,
    Popcnt,
    Eqz,

    // Float
    FAbs,
    FNeg,
    FCeil,
    FFloor,
    FTrunc,
    FNearest,
    FSqrt,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    LtS,
    LtU,
    GtS,
    GtU,
    LeS,
    LeU,
    GeS,
    GeU,

    // Float
    FEq,
    FNe,
    FLt,
    FGt,
    FLe,
    FGe,
}

/// Type conversion operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertOp {
    I32WrapI64,
    I64ExtendI32S,
    I64ExtendI32U,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F32DemoteF64,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,
    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,
    I32TruncSatF32S,
    I32TruncSatF32U,
    I32TruncSatF64S,
    I32TruncSatF64U,
    I64TruncSatF32S,
    I64TruncSatF32U,
    I64TruncSatF64S,
    I64TruncSatF64U,
}

/// Inferred type for expressions
#[derive(Debug, Clone, PartialEq)]
pub enum InferredType {
    Unknown,
    I32,
    I64,
    F32,
    F64,
    /// Pointer to another type
    Pointer(Box<InferredType>),
    /// Go string (ptr, len)
    GoString,
    /// Go slice (ptr, len, cap)
    GoSlice(Box<InferredType>),
    /// Go interface (type, data)
    GoInterface,
    /// C null-terminated string
    CString,
    /// Array of T with known size
    Array(Box<InferredType>, usize),
    /// Boolean (result of comparison)
    Bool,
}

impl InferredType {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::I32 | Self::I64 | Self::F32 | Self::F64)
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Self::I32 | Self::I64)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
}
