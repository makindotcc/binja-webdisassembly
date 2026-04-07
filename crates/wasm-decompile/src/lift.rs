//! WASM bytecode to IR lifting
//!
//! This module converts raw WASM bytecode into our intermediate representation.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use wasmparser::{
    BinaryReader, BlockType, DataSectionReader, ElementSectionReader, ExportSectionReader,
    FunctionBody, FunctionSectionReader, GlobalSectionReader, ImportSectionReader,
    MemorySectionReader, NameSectionReader, Operator, Parser, Payload, TypeSectionReader,
    ValType as WasmValType,
};

use crate::ir::*;

/// Lift WASM bytecode to IR Module
pub fn lift(wasm: &[u8]) -> Result<Module> {
    let mut module = Module::new();
    let parser = Parser::new(0);

    let mut code_bodies: Vec<FunctionBody> = Vec::new();
    let mut num_imported_funcs = 0u32;
    let mut func_names: HashMap<u32, String> = HashMap::new();

    for payload in parser.parse_all(wasm) {
        match payload? {
            Payload::TypeSection(reader) => {
                parse_types(&mut module, reader)?;
            }
            Payload::ImportSection(reader) => {
                num_imported_funcs = parse_imports(&mut module, reader)?;
            }
            Payload::FunctionSection(reader) => {
                parse_functions(&mut module, reader)?;
            }
            Payload::GlobalSection(reader) => {
                parse_globals(&mut module, reader)?;
            }
            Payload::ExportSection(reader) => {
                parse_exports(&mut module, reader)?;
            }
            Payload::MemorySection(reader) => {
                parse_memory(&mut module, reader)?;
            }
            Payload::DataSection(reader) => {
                parse_data(&mut module, reader)?;
            }
            Payload::CodeSectionEntry(body) => {
                code_bodies.push(body);
            }
            Payload::ElementSection(reader) => {
                parse_elements(&mut module, reader)?;
            }
            Payload::CustomSection(section) if section.name() == "name" => {
                parse_name_section(&mut func_names, section.data())?;
            }
            _ => {}
        }
    }

    // Deduplicate function names - track which names are used and append index for duplicates
    let unique_names = deduplicate_func_names(&func_names, &module.exports);

    // Now lift all function bodies
    for (i, body) in code_bodies.into_iter().enumerate() {
        let func_idx = num_imported_funcs + i as u32;
        let func = lift_function(&module, func_idx, body, &unique_names)?;
        module.functions.push(func);
    }

    // Also update imported function names from name section
    for func in &mut module.functions {
        if func.is_import && func.name.is_none() {
            if let Some(name) = unique_names.get(&func.index) {
                func.name = Some(name.clone());
            }
        }
    }

    Ok(module)
}

/// Deduplicate function names by appending function index for duplicates.
/// Export names take priority over name section names.
fn deduplicate_func_names(
    func_names: &HashMap<u32, String>,
    exports: &HashMap<u32, String>,
) -> HashMap<u32, String> {
    use std::collections::HashSet;

    let mut result = HashMap::new();
    let mut used_names: HashSet<String> = HashSet::new();

    // Collect all (func_idx, name) pairs, prioritizing exports
    let mut all_names: Vec<(u32, String)> = Vec::new();
    for (idx, name) in func_names.iter() {
        let final_name = exports.get(idx).cloned().unwrap_or_else(|| name.clone());
        all_names.push((*idx, final_name));
    }
    // Also include exports that might not be in func_names
    for (idx, name) in exports.iter() {
        if !func_names.contains_key(idx) {
            all_names.push((*idx, name.clone()));
        }
    }

    // Sort by function index to ensure deterministic ordering
    all_names.sort_by_key(|(idx, _)| *idx);

    // Assign unique names
    for (idx, name) in all_names {
        let unique_name = if used_names.contains(&name) {
            // Name collision - append function index
            format!("{}_{}", name, idx)
        } else {
            name.clone()
        };
        used_names.insert(unique_name.clone());
        result.insert(idx, unique_name);
    }

    result
}

fn parse_name_section(func_names: &mut HashMap<u32, String>, data: &[u8]) -> Result<()> {
    let reader = NameSectionReader::new(BinaryReader::new(data, 0));
    for name in reader {
        if let Ok(wasmparser::Name::Function(map)) = name {
            for naming in map {
                if let Ok(naming) = naming {
                    func_names.insert(naming.index, naming.name.to_string());
                }
            }
        }
    }
    Ok(())
}

fn parse_types(module: &mut Module, reader: TypeSectionReader) -> Result<()> {
    for rec_group in reader {
        let rec_group = rec_group?;
        for ty in rec_group.into_types() {
            if let wasmparser::CompositeInnerType::Func(func_type) = ty.composite_type.inner {
                let params: Vec<ValType> = func_type
                    .params()
                    .iter()
                    .filter_map(|t| convert_val_type(*t))
                    .collect();
                let results: Vec<ValType> = func_type
                    .results()
                    .iter()
                    .filter_map(|t| convert_val_type(*t))
                    .collect();
                module.types.push(FuncType { params, results });
            }
        }
    }
    Ok(())
}

fn parse_imports(module: &mut Module, reader: ImportSectionReader) -> Result<u32> {
    let mut num_funcs = 0u32;

    for import in reader {
        let import = import?;
        let kind = match import.ty {
            wasmparser::TypeRef::Func(idx) => {
                num_funcs += 1;
                module.func_types.push(idx);

                // Create a placeholder function for imports
                if let Some(ft) = module.types.get(idx as usize) {
                    module.functions.push(Function {
                        index: num_funcs - 1,
                        name: Some(format!("{}_{}", import.module, import.name)),
                        params: ft.params.clone(),
                        results: ft.results.clone(),
                        locals: Vec::new(),
                        body: Block::new(),
                        is_import: true,
                    });
                }

                ImportKind::Function(idx)
            }
            wasmparser::TypeRef::Global(g) => {
                ImportKind::Global(convert_val_type(g.content_type).unwrap_or(ValType::I32))
            }
            wasmparser::TypeRef::Memory(_) => ImportKind::Memory,
            wasmparser::TypeRef::Table(_) => ImportKind::Table,
            wasmparser::TypeRef::Tag(_) => continue,
        };

        module.imports.push(Import {
            module: import.module.to_string(),
            name: import.name.to_string(),
            kind,
        });
    }

    Ok(num_funcs)
}

fn parse_functions(module: &mut Module, reader: FunctionSectionReader) -> Result<()> {
    for type_idx in reader {
        module.func_types.push(type_idx?);
    }
    Ok(())
}

fn parse_globals(module: &mut Module, reader: GlobalSectionReader) -> Result<()> {
    for global in reader {
        let global = global?;
        let ty = convert_val_type(global.ty.content_type).unwrap_or(ValType::I32);
        let mutable = global.ty.mutable;

        // Parse the init expression
        let init = parse_const_expr(global.init_expr.get_binary_reader())?;

        module.globals.push(Global { ty, mutable, init });
    }
    Ok(())
}

fn parse_exports(module: &mut Module, reader: ExportSectionReader) -> Result<()> {
    for export in reader {
        let export = export?;
        if let wasmparser::ExternalKind::Func = export.kind {
            module.exports.insert(export.index, export.name.to_string());
        }
    }
    Ok(())
}

fn parse_memory(module: &mut Module, reader: MemorySectionReader) -> Result<()> {
    for memory in reader {
        let memory = memory?;
        // Initialize memory to minimum size
        let min_pages = memory.initial as usize;
        module.memory_pages = min_pages as u32;
        module.memory.resize(min_pages * 65536, 0);
    }
    Ok(())
}

fn parse_data(module: &mut Module, reader: DataSectionReader) -> Result<()> {
    for data in reader {
        let data = data?;
        match data.kind {
            wasmparser::DataKind::Active {
                memory_index: _,
                offset_expr,
            } => {
                let offset = eval_const_expr(offset_expr.get_binary_reader())? as usize;
                let data_bytes = data.data;

                // Ensure memory is large enough
                let end = offset + data_bytes.len();
                if end > module.memory.len() {
                    module.memory.resize(end, 0);
                }

                // Copy data into memory
                module.memory[offset..end].copy_from_slice(data_bytes);

                module.data_segments.push(DataSegment {
                    offset: offset as u32,
                    data: data_bytes.to_vec(),
                });
            }
            wasmparser::DataKind::Passive => {
                // Passive data segments are not copied to memory initially
                module.data_segments.push(DataSegment {
                    offset: 0,
                    data: data.data.to_vec(),
                });
            }
        }
    }
    Ok(())
}

fn parse_elements(module: &mut Module, reader: ElementSectionReader) -> Result<()> {
    use wasmparser::ElementItems;

    for elem in reader {
        let elem = elem?;
        // We only handle active elements (offset into table 0)
        if let wasmparser::ElementKind::Active {
            table_index,
            offset_expr,
        } = elem.kind
        {
            // Only table 0 supported
            let table = table_index.unwrap_or(0);
            if table != 0 {
                continue;
            }
            let offset = eval_const_expr(offset_expr.get_binary_reader())? as u32;

            let mut func_indices = Vec::new();
            match elem.items {
                ElementItems::Functions(reader) => {
                    for func_idx in reader {
                        func_indices.push(func_idx?);
                    }
                }
                ElementItems::Expressions(_ref_type, reader) => {
                    for expr in reader {
                        let expr = expr?;
                        let mut expr_reader = expr.get_binary_reader();
                        let mut idx = 0u32;
                        while !expr_reader.eof() {
                            match expr_reader.read_operator()? {
                                Operator::RefFunc { function_index } => idx = function_index,
                                Operator::End => break,
                                _ => {}
                            }
                        }
                        func_indices.push(idx);
                    }
                }
            }

            module.elements.push(ElementSegment {
                offset,
                func_indices,
            });
        }
    }
    Ok(())
}

fn parse_const_expr(mut reader: BinaryReader) -> Result<Expr> {
    while !reader.eof() {
        let op = reader.read_operator()?;
        match op {
            Operator::I32Const { value } => return Ok(Expr::i32_const(value)),
            Operator::I64Const { value } => return Ok(Expr::i64_const(value)),
            Operator::F32Const { value } => {
                return Ok(Expr::f32_const(f32::from_bits(value.bits())))
            }
            Operator::F64Const { value } => {
                return Ok(Expr::f64_const(f64::from_bits(value.bits())))
            }
            Operator::GlobalGet { global_index } => return Ok(Expr::global(global_index)),
            Operator::End => break,
            _ => {}
        }
    }
    Ok(Expr::i32_const(0))
}

fn eval_const_expr(mut reader: BinaryReader) -> Result<i64> {
    while !reader.eof() {
        let op = reader.read_operator()?;
        match op {
            Operator::I32Const { value } => return Ok(value as i64),
            Operator::I64Const { value } => return Ok(value),
            Operator::End => break,
            _ => {}
        }
    }
    Ok(0)
}

fn convert_val_type(ty: WasmValType) -> Option<ValType> {
    match ty {
        WasmValType::I32 => Some(ValType::I32),
        WasmValType::I64 => Some(ValType::I64),
        WasmValType::F32 => Some(ValType::F32),
        WasmValType::F64 => Some(ValType::F64),
        _ => None,
    }
}

/// Lift a single function body to IR
fn lift_function(
    module: &Module,
    func_idx: u32,
    body: FunctionBody,
    func_names: &HashMap<u32, String>,
) -> Result<Function> {
    let type_idx = module
        .func_types
        .get(func_idx as usize)
        .ok_or_else(|| anyhow!("Missing type for function {}", func_idx))?;
    let func_type = module
        .types
        .get(*type_idx as usize)
        .ok_or_else(|| anyhow!("Missing function type {}", type_idx))?;

    let mut locals_reader = body.get_locals_reader()?;
    let mut locals = Vec::new();

    for _ in 0..locals_reader.get_count() {
        let (count, ty) = locals_reader.read()?;
        if let Some(vt) = convert_val_type(ty) {
            for _ in 0..count {
                locals.push(vt);
            }
        }
    }

    // Use deduplicated name from func_names (already includes exports with deduplication)
    let name = func_names.get(&func_idx).cloned();

    let mut lifter = FunctionLifter::new(func_type.params.len() as u32, &module.types, &module.func_types);

    let ops_reader = body.get_operators_reader()?;
    for op in ops_reader {
        lifter.process_op(op?)?;
    }

    let body = lifter.finish();

    Ok(Function {
        index: func_idx,
        name,
        params: func_type.params.clone(),
        results: func_type.results.clone(),
        locals,
        body,
        is_import: false,
    })
}

/// Function lifter - converts WASM operators to IR
struct FunctionLifter<'a> {
    /// Value stack (simulates WASM stack)
    stack: Vec<Expr>,
    /// Statement list for current block
    stmts: Vec<Stmt>,
    /// Control flow stack (for blocks/loops/ifs)
    control_stack: Vec<ControlFrame>,
    /// Next label ID
    next_label: u32,
    /// Number of parameters (to distinguish params from locals)
    #[allow(dead_code)]
    num_params: u32,
    /// Next temp local index for saving locals before reassignment
    next_temp_local: u32,
    /// Function type signatures (from module)
    types: &'a [FuncType],
    /// Function index -> type index mapping (from module)
    func_types: &'a [u32],
}

struct ControlFrame {
    kind: ControlKind,
    label: u32,
    stmts: Vec<Stmt>,
    else_stmts: Option<Vec<Stmt>>,
    /// Block type — used to detect if/else that produces a value
    blockty: BlockType,
    /// Saved value stack depth at entry (to extract result values)
    stack_depth_at_entry: usize,
    /// Values from the true branch of an if/else with result type
    then_result: Option<Vec<Expr>>,
    /// Saved condition for If blocks (not stored on the value stack)
    saved_cond: Option<Expr>,
    /// Temp local for typed block results carried by br/br_if
    block_result_local: Option<u32>,
}

#[derive(Clone, Copy)]
enum ControlKind {
    Block,
    Loop,
    If,
}

/// Check if an expression tree contains a reference to Local(idx)
fn expr_contains_local(expr: &Expr, idx: u32) -> bool {
    match &expr.kind {
        ExprKind::Local(i) => *i == idx,
        ExprKind::BinOp(_, a, b) | ExprKind::Compare(_, a, b, _) => {
            expr_contains_local(a, idx) || expr_contains_local(b, idx)
        }
        ExprKind::UnaryOp(_, e) | ExprKind::Convert { expr: e, .. } => {
            expr_contains_local(e, idx)
        }
        ExprKind::Load { addr, .. } => expr_contains_local(addr, idx),
        ExprKind::Call { args, .. } => args.iter().any(|a| expr_contains_local(a, idx)),
        ExprKind::CallIndirect { index, args, .. } => {
            expr_contains_local(index, idx) || args.iter().any(|a| expr_contains_local(a, idx))
        }
        ExprKind::Select { cond, then_val, else_val } => {
            expr_contains_local(cond, idx)
                || expr_contains_local(then_val, idx)
                || expr_contains_local(else_val, idx)
        }
        _ => false,
    }
}

/// Replace all Local(old_idx) references with Local(new_idx) in an expression tree
fn expr_replace_local(expr: &mut Expr, old_idx: u32, new_idx: u32) {
    match &mut expr.kind {
        ExprKind::Local(i) if *i == old_idx => *i = new_idx,
        ExprKind::BinOp(_, a, b) | ExprKind::Compare(_, a, b, _) => {
            expr_replace_local(a, old_idx, new_idx);
            expr_replace_local(b, old_idx, new_idx);
        }
        ExprKind::UnaryOp(_, e) | ExprKind::Convert { expr: e, .. } => {
            expr_replace_local(e, old_idx, new_idx);
        }
        ExprKind::Load { addr, .. } => expr_replace_local(addr, old_idx, new_idx),
        ExprKind::Call { args, .. } => {
            for a in args.iter_mut() {
                expr_replace_local(a, old_idx, new_idx);
            }
        }
        ExprKind::CallIndirect { index, args, .. } => {
            expr_replace_local(index, old_idx, new_idx);
            for a in args.iter_mut() {
                expr_replace_local(a, old_idx, new_idx);
            }
        }
        ExprKind::Select { cond, then_val, else_val } => {
            expr_replace_local(cond, old_idx, new_idx);
            expr_replace_local(then_val, old_idx, new_idx);
            expr_replace_local(else_val, old_idx, new_idx);
        }
        _ => {}
    }
}

impl<'a> FunctionLifter<'a> {
    fn new(num_params: u32, types: &'a [FuncType], func_types: &'a [u32]) -> Self {
        Self {
            stack: Vec::new(),
            stmts: Vec::new(),
            control_stack: Vec::new(),
            next_label: 0,
            num_params,
            next_temp_local: u32::MAX / 2,
            types,
            func_types,
        }
    }

    /// Look up a function's type signature by function index
    fn get_func_type(&self, func_idx: u32) -> Option<&'a FuncType> {
        let type_idx = self.func_types.get(func_idx as usize)?;
        self.types.get(*type_idx as usize)
    }

    fn pop(&mut self) -> Expr {
        self.stack.pop().unwrap_or_else(|| Expr::i32_const(0))
    }

    fn pop2(&mut self) -> (Expr, Expr) {
        let b = self.pop();
        let a = self.pop();
        (a, b)
    }

    fn push(&mut self, expr: Expr) {
        self.stack.push(expr);
    }

    fn emit(&mut self, stmt: Stmt) {
        self.stmts.push(stmt);
    }

    /// If any expression on the stack references Local(idx), save the old value
    /// in a temp local and replace all stack references to use the temp.
    /// This prevents correctness issues when local.set/local.tee overwrites a local
    /// that is already referenced by pending stack expressions.
    fn save_local_if_conflicting(&mut self, idx: u32) {
        let has_conflict = self.stack.iter().any(|e| expr_contains_local(e, idx));
        if has_conflict {
            let temp = self.next_temp_local;
            self.next_temp_local += 1;
            self.emit(Stmt::LocalSet {
                local: temp,
                value: Expr::local(idx),
            });
            for expr in &mut self.stack {
                expr_replace_local(expr, idx, temp);
            }
        }
    }

    /// How many result values a block type produces
    fn block_result_count(&self, blockty: BlockType) -> usize {
        match blockty {
            BlockType::Empty => 0,
            BlockType::Type(_) => 1,
            BlockType::FuncType(idx) => {
                self.types
                    .get(idx as usize)
                    .map(|ft| ft.results.len())
                    .unwrap_or(0)
            }
        }
    }

    fn alloc_label(&mut self) -> u32 {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    fn process_op(&mut self, op: Operator) -> Result<()> {
        use Operator::*;

        match op {
            // Constants
            I32Const { value } => self.push(Expr::i32_const(value)),
            I64Const { value } => self.push(Expr::i64_const(value)),
            F32Const { value } => self.push(Expr::f32_const(f32::from_bits(value.bits()))),
            F64Const { value } => self.push(Expr::f64_const(f64::from_bits(value.bits()))),

            // Local variables
            LocalGet { local_index } => {
                self.push(Expr::local(local_index));
            }
            LocalSet { local_index } => {
                let value = self.pop();
                self.save_local_if_conflicting(local_index);
                self.emit(Stmt::LocalSet {
                    local: local_index,
                    value,
                });
            }
            LocalTee { local_index } => {
                let value = self.pop();
                self.save_local_if_conflicting(local_index);
                self.emit(Stmt::LocalSet {
                    local: local_index,
                    value,
                });
                // Push reference to the local (now holds the value)
                // This is important: we push Local(idx), not the original expression,
                // because the expression might reference variables that get modified.
                self.push(Expr::local(local_index));
            }

            // Global variables
            GlobalGet { global_index } => {
                self.push(Expr::global(global_index));
            }
            GlobalSet { global_index } => {
                let value = self.pop();
                self.emit(Stmt::GlobalSet {
                    global: global_index,
                    value,
                });
            }

            // Memory operations
            I32Load { memarg } => self.lift_load(memarg.offset as u32, MemSize::I32, false),
            I64Load { memarg } => self.lift_load(memarg.offset as u32, MemSize::I64, false),
            F32Load { memarg } => self.lift_load(memarg.offset as u32, MemSize::F32, false),
            F64Load { memarg } => self.lift_load(memarg.offset as u32, MemSize::F64, false),
            I32Load8S { memarg } => self.lift_load(memarg.offset as u32, MemSize::I8, true),
            I32Load8U { memarg } => self.lift_load(memarg.offset as u32, MemSize::I8, false),
            I32Load16S { memarg } => self.lift_load(memarg.offset as u32, MemSize::I16, true),
            I32Load16U { memarg } => self.lift_load(memarg.offset as u32, MemSize::I16, false),
            I64Load8S { memarg } => self.lift_load(memarg.offset as u32, MemSize::I8, true),
            I64Load8U { memarg } => self.lift_load(memarg.offset as u32, MemSize::I8, false),
            I64Load16S { memarg } => self.lift_load(memarg.offset as u32, MemSize::I16, true),
            I64Load16U { memarg } => self.lift_load(memarg.offset as u32, MemSize::I16, false),
            I64Load32S { memarg } => self.lift_load(memarg.offset as u32, MemSize::I32, true),
            I64Load32U { memarg } => self.lift_load(memarg.offset as u32, MemSize::I32, false),

            I32Store { memarg } => self.lift_store(memarg.offset as u32, MemSize::I32),
            I64Store { memarg } => self.lift_store(memarg.offset as u32, MemSize::I64),
            F32Store { memarg } => self.lift_store(memarg.offset as u32, MemSize::F32),
            F64Store { memarg } => self.lift_store(memarg.offset as u32, MemSize::F64),
            I32Store8 { memarg } => self.lift_store(memarg.offset as u32, MemSize::I8),
            I32Store16 { memarg } => self.lift_store(memarg.offset as u32, MemSize::I16),
            I64Store8 { memarg } => self.lift_store(memarg.offset as u32, MemSize::I8),
            I64Store16 { memarg } => self.lift_store(memarg.offset as u32, MemSize::I16),
            I64Store32 { memarg } => self.lift_store(memarg.offset as u32, MemSize::I32),

            // Binary operations (i32)
            I32Add => self.lift_binop(BinOp::Add, InferredType::I32),
            I32Sub => self.lift_binop(BinOp::Sub, InferredType::I32),
            I32Mul => self.lift_binop(BinOp::Mul, InferredType::I32),
            I32DivS => self.lift_binop(BinOp::DivS, InferredType::I32),
            I32DivU => self.lift_binop(BinOp::DivU, InferredType::I32),
            I32RemS => self.lift_binop(BinOp::RemS, InferredType::I32),
            I32RemU => self.lift_binop(BinOp::RemU, InferredType::I32),
            I32And => self.lift_binop(BinOp::And, InferredType::I32),
            I32Or => self.lift_binop(BinOp::Or, InferredType::I32),
            I32Xor => self.lift_binop(BinOp::Xor, InferredType::I32),
            I32Shl => self.lift_binop(BinOp::Shl, InferredType::I32),
            I32ShrS => self.lift_binop(BinOp::ShrS, InferredType::I32),
            I32ShrU => self.lift_binop(BinOp::ShrU, InferredType::I32),
            I32Rotl => self.lift_binop(BinOp::Rotl, InferredType::I32),
            I32Rotr => self.lift_binop(BinOp::Rotr, InferredType::I32),

            // Binary operations (i64)
            I64Add => self.lift_binop(BinOp::Add, InferredType::I64),
            I64Sub => self.lift_binop(BinOp::Sub, InferredType::I64),
            I64Mul => self.lift_binop(BinOp::Mul, InferredType::I64),
            I64DivS => self.lift_binop(BinOp::DivS, InferredType::I64),
            I64DivU => self.lift_binop(BinOp::DivU, InferredType::I64),
            I64RemS => self.lift_binop(BinOp::RemS, InferredType::I64),
            I64RemU => self.lift_binop(BinOp::RemU, InferredType::I64),
            I64And => self.lift_binop(BinOp::And, InferredType::I64),
            I64Or => self.lift_binop(BinOp::Or, InferredType::I64),
            I64Xor => self.lift_binop(BinOp::Xor, InferredType::I64),
            I64Shl => self.lift_binop(BinOp::Shl, InferredType::I64),
            I64ShrS => self.lift_binop(BinOp::ShrS, InferredType::I64),
            I64ShrU => self.lift_binop(BinOp::ShrU, InferredType::I64),
            I64Rotl => self.lift_binop(BinOp::Rotl, InferredType::I64),
            I64Rotr => self.lift_binop(BinOp::Rotr, InferredType::I64),

            // Float binary operations
            F32Add => self.lift_binop(BinOp::FAdd, InferredType::F32),
            F32Sub => self.lift_binop(BinOp::FSub, InferredType::F32),
            F32Mul => self.lift_binop(BinOp::FMul, InferredType::F32),
            F32Div => self.lift_binop(BinOp::FDiv, InferredType::F32),
            F32Min => self.lift_binop(BinOp::FMin, InferredType::F32),
            F32Max => self.lift_binop(BinOp::FMax, InferredType::F32),
            F32Copysign => self.lift_binop(BinOp::FCopysign, InferredType::F32),

            F64Add => self.lift_binop(BinOp::FAdd, InferredType::F64),
            F64Sub => self.lift_binop(BinOp::FSub, InferredType::F64),
            F64Mul => self.lift_binop(BinOp::FMul, InferredType::F64),
            F64Div => self.lift_binop(BinOp::FDiv, InferredType::F64),
            F64Min => self.lift_binop(BinOp::FMin, InferredType::F64),
            F64Max => self.lift_binop(BinOp::FMax, InferredType::F64),
            F64Copysign => self.lift_binop(BinOp::FCopysign, InferredType::F64),

            // Unary operations
            I32Clz => self.lift_unaryop(UnaryOp::Clz, InferredType::I32),
            I32Ctz => self.lift_unaryop(UnaryOp::Ctz, InferredType::I32),
            I32Popcnt => self.lift_unaryop(UnaryOp::Popcnt, InferredType::I32),
            I32Eqz => self.lift_unaryop(UnaryOp::Eqz, InferredType::I32),
            I64Clz => self.lift_unaryop(UnaryOp::Clz, InferredType::I64),
            I64Ctz => self.lift_unaryop(UnaryOp::Ctz, InferredType::I64),
            I64Popcnt => self.lift_unaryop(UnaryOp::Popcnt, InferredType::I64),
            I64Eqz => self.lift_unaryop(UnaryOp::Eqz, InferredType::I64),

            F32Abs => self.lift_unaryop(UnaryOp::FAbs, InferredType::F32),
            F32Neg => self.lift_unaryop(UnaryOp::FNeg, InferredType::F32),
            F32Ceil => self.lift_unaryop(UnaryOp::FCeil, InferredType::F32),
            F32Floor => self.lift_unaryop(UnaryOp::FFloor, InferredType::F32),
            F32Trunc => self.lift_unaryop(UnaryOp::FTrunc, InferredType::F32),
            F32Nearest => self.lift_unaryop(UnaryOp::FNearest, InferredType::F32),
            F32Sqrt => self.lift_unaryop(UnaryOp::FSqrt, InferredType::F32),

            F64Abs => self.lift_unaryop(UnaryOp::FAbs, InferredType::F64),
            F64Neg => self.lift_unaryop(UnaryOp::FNeg, InferredType::F64),
            F64Ceil => self.lift_unaryop(UnaryOp::FCeil, InferredType::F64),
            F64Floor => self.lift_unaryop(UnaryOp::FFloor, InferredType::F64),
            F64Trunc => self.lift_unaryop(UnaryOp::FTrunc, InferredType::F64),
            F64Nearest => self.lift_unaryop(UnaryOp::FNearest, InferredType::F64),
            F64Sqrt => self.lift_unaryop(UnaryOp::FSqrt, InferredType::F64),

            // Comparisons (i32)
            I32Eq => self.lift_cmpop(CmpOp::Eq, InferredType::I32),
            I32Ne => self.lift_cmpop(CmpOp::Ne, InferredType::I32),
            I32LtS => self.lift_cmpop(CmpOp::LtS, InferredType::I32),
            I32LtU => self.lift_cmpop(CmpOp::LtU, InferredType::I32),
            I32GtS => self.lift_cmpop(CmpOp::GtS, InferredType::I32),
            I32GtU => self.lift_cmpop(CmpOp::GtU, InferredType::I32),
            I32LeS => self.lift_cmpop(CmpOp::LeS, InferredType::I32),
            I32LeU => self.lift_cmpop(CmpOp::LeU, InferredType::I32),
            I32GeS => self.lift_cmpop(CmpOp::GeS, InferredType::I32),
            I32GeU => self.lift_cmpop(CmpOp::GeU, InferredType::I32),

            // Comparisons (i64)
            I64Eq => self.lift_cmpop(CmpOp::Eq, InferredType::I64),
            I64Ne => self.lift_cmpop(CmpOp::Ne, InferredType::I64),
            I64LtS => self.lift_cmpop(CmpOp::LtS, InferredType::I64),
            I64LtU => self.lift_cmpop(CmpOp::LtU, InferredType::I64),
            I64GtS => self.lift_cmpop(CmpOp::GtS, InferredType::I64),
            I64GtU => self.lift_cmpop(CmpOp::GtU, InferredType::I64),
            I64LeS => self.lift_cmpop(CmpOp::LeS, InferredType::I64),
            I64LeU => self.lift_cmpop(CmpOp::LeU, InferredType::I64),
            I64GeS => self.lift_cmpop(CmpOp::GeS, InferredType::I64),
            I64GeU => self.lift_cmpop(CmpOp::GeU, InferredType::I64),

            // Float comparisons
            F32Eq => self.lift_cmpop(CmpOp::FEq, InferredType::F32),
            F32Ne => self.lift_cmpop(CmpOp::FNe, InferredType::F32),
            F32Lt => self.lift_cmpop(CmpOp::FLt, InferredType::F32),
            F32Gt => self.lift_cmpop(CmpOp::FGt, InferredType::F32),
            F32Le => self.lift_cmpop(CmpOp::FLe, InferredType::F32),
            F32Ge => self.lift_cmpop(CmpOp::FGe, InferredType::F32),

            F64Eq => self.lift_cmpop(CmpOp::FEq, InferredType::F64),
            F64Ne => self.lift_cmpop(CmpOp::FNe, InferredType::F64),
            F64Lt => self.lift_cmpop(CmpOp::FLt, InferredType::F64),
            F64Gt => self.lift_cmpop(CmpOp::FGt, InferredType::F64),
            F64Le => self.lift_cmpop(CmpOp::FLe, InferredType::F64),
            F64Ge => self.lift_cmpop(CmpOp::FGe, InferredType::F64),

            // Conversions
            I32WrapI64 => self.lift_convert(ConvertOp::I32WrapI64, InferredType::I32),
            I64ExtendI32S => self.lift_convert(ConvertOp::I64ExtendI32S, InferredType::I64),
            I64ExtendI32U => self.lift_convert(ConvertOp::I64ExtendI32U, InferredType::I64),
            I32TruncF32S => self.lift_convert(ConvertOp::I32TruncF32S, InferredType::I32),
            I32TruncF32U => self.lift_convert(ConvertOp::I32TruncF32U, InferredType::I32),
            I32TruncF64S => self.lift_convert(ConvertOp::I32TruncF64S, InferredType::I32),
            I32TruncF64U => self.lift_convert(ConvertOp::I32TruncF64U, InferredType::I32),
            I64TruncF32S => self.lift_convert(ConvertOp::I64TruncF32S, InferredType::I64),
            I64TruncF32U => self.lift_convert(ConvertOp::I64TruncF32U, InferredType::I64),
            I64TruncF64S => self.lift_convert(ConvertOp::I64TruncF64S, InferredType::I64),
            I64TruncF64U => self.lift_convert(ConvertOp::I64TruncF64U, InferredType::I64),
            F32ConvertI32S => self.lift_convert(ConvertOp::F32ConvertI32S, InferredType::F32),
            F32ConvertI32U => self.lift_convert(ConvertOp::F32ConvertI32U, InferredType::F32),
            F32ConvertI64S => self.lift_convert(ConvertOp::F32ConvertI64S, InferredType::F32),
            F32ConvertI64U => self.lift_convert(ConvertOp::F32ConvertI64U, InferredType::F32),
            F64ConvertI32S => self.lift_convert(ConvertOp::F64ConvertI32S, InferredType::F64),
            F64ConvertI32U => self.lift_convert(ConvertOp::F64ConvertI32U, InferredType::F64),
            F64ConvertI64S => self.lift_convert(ConvertOp::F64ConvertI64S, InferredType::F64),
            F64ConvertI64U => self.lift_convert(ConvertOp::F64ConvertI64U, InferredType::F64),
            F32DemoteF64 => self.lift_convert(ConvertOp::F32DemoteF64, InferredType::F32),
            F64PromoteF32 => self.lift_convert(ConvertOp::F64PromoteF32, InferredType::F64),
            I32ReinterpretF32 => self.lift_convert(ConvertOp::I32ReinterpretF32, InferredType::I32),
            I64ReinterpretF64 => self.lift_convert(ConvertOp::I64ReinterpretF64, InferredType::I64),
            F32ReinterpretI32 => self.lift_convert(ConvertOp::F32ReinterpretI32, InferredType::F32),
            F64ReinterpretI64 => self.lift_convert(ConvertOp::F64ReinterpretI64, InferredType::F64),
            I32Extend8S => self.lift_convert(ConvertOp::I32Extend8S, InferredType::I32),
            I32Extend16S => self.lift_convert(ConvertOp::I32Extend16S, InferredType::I32),
            I64Extend8S => self.lift_convert(ConvertOp::I64Extend8S, InferredType::I64),
            I64Extend16S => self.lift_convert(ConvertOp::I64Extend16S, InferredType::I64),
            I64Extend32S => self.lift_convert(ConvertOp::I64Extend32S, InferredType::I64),
            I32TruncSatF32S => self.lift_convert(ConvertOp::I32TruncSatF32S, InferredType::I32),
            I32TruncSatF32U => self.lift_convert(ConvertOp::I32TruncSatF32U, InferredType::I32),
            I32TruncSatF64S => self.lift_convert(ConvertOp::I32TruncSatF64S, InferredType::I32),
            I32TruncSatF64U => self.lift_convert(ConvertOp::I32TruncSatF64U, InferredType::I32),
            I64TruncSatF32S => self.lift_convert(ConvertOp::I64TruncSatF32S, InferredType::I64),
            I64TruncSatF32U => self.lift_convert(ConvertOp::I64TruncSatF32U, InferredType::I64),
            I64TruncSatF64S => self.lift_convert(ConvertOp::I64TruncSatF64S, InferredType::I64),
            I64TruncSatF64U => self.lift_convert(ConvertOp::I64TruncSatF64U, InferredType::I64),

            // Calls
            Call { function_index } => {
                let func_type = self.get_func_type(function_index);
                let num_params = func_type.map(|t| t.params.len()).unwrap_or(0);
                let has_result = func_type.map(|t| !t.results.is_empty()).unwrap_or(true);

                // Pop exactly the right number of arguments from the stack
                let stack_len = self.stack.len();
                let args = if num_params <= stack_len {
                    self.stack.split_off(stack_len - num_params)
                } else {
                    std::mem::take(&mut self.stack)
                };

                let call_expr = Expr::new(ExprKind::Call {
                    func: function_index,
                    args,
                });

                if has_result {
                    self.push(call_expr);
                } else {
                    self.emit(Stmt::Expr(call_expr));
                }
            }

            CallIndirect {
                type_index,
                table_index,
            } => {
                let index = self.pop();
                let func_type = self.types.get(type_index as usize);
                let num_params = func_type.map(|t| t.params.len()).unwrap_or(0);
                let has_result = func_type.map(|t| !t.results.is_empty()).unwrap_or(true);

                let stack_len = self.stack.len();
                let args = if num_params <= stack_len {
                    self.stack.split_off(stack_len - num_params)
                } else {
                    std::mem::take(&mut self.stack)
                };

                let call_expr = Expr::new(ExprKind::CallIndirect {
                    type_idx: type_index,
                    table_idx: table_index,
                    index: Box::new(index),
                    args,
                });

                if has_result {
                    self.push(call_expr);
                } else {
                    self.emit(Stmt::Expr(call_expr));
                }
            }

            // Control flow
            Block { blockty } => {
                let label = self.alloc_label();
                let stack_depth = self.stack.len();
                self.control_stack.push(ControlFrame {
                    kind: ControlKind::Block,
                    label,
                    stmts: std::mem::take(&mut self.stmts),
                    else_stmts: None,
                    blockty,
                    stack_depth_at_entry: stack_depth,
                    then_result: None,
                    saved_cond: None,
                    block_result_local: None,
                });
            }

            Loop { blockty } => {
                let label = self.alloc_label();
                let stack_depth = self.stack.len();
                self.control_stack.push(ControlFrame {
                    kind: ControlKind::Loop,
                    label,
                    stmts: std::mem::take(&mut self.stmts),
                    else_stmts: None,
                    blockty,
                    stack_depth_at_entry: stack_depth,
                    then_result: None,
                    saved_cond: None,
                    block_result_local: None,
                });
            }

            If { blockty } => {
                let cond = self.pop();
                let label = self.alloc_label();
                let stack_depth = self.stack.len();
                self.control_stack.push(ControlFrame {
                    kind: ControlKind::If,
                    label,
                    stmts: std::mem::take(&mut self.stmts),
                    else_stmts: None,
                    blockty,
                    stack_depth_at_entry: stack_depth,
                    then_result: None,
                    saved_cond: Some(cond),
                    block_result_local: None,
                });
            }

            Else => {
                // Compute result count before mutably borrowing the frame
                let result_count = self
                    .control_stack
                    .last()
                    .map(|f| self.block_result_count(f.blockty))
                    .unwrap_or(0);
                if let Some(frame) = self.control_stack.last_mut() {
                    // Save result values from the true branch
                    if result_count > 0 {
                        let drain_start = self.stack.len().saturating_sub(result_count);
                        frame.then_result =
                            Some(self.stack.drain(drain_start..).collect());
                    }
                    frame.else_stmts = Some(std::mem::take(&mut self.stmts));
                }
            }

            End => {
                if let Some(frame) = self.control_stack.pop() {
                    let body = crate::ir::Block::with_stmts(std::mem::take(&mut self.stmts));
                    self.stmts = frame.stmts;

                    match frame.kind {
                        ControlKind::Block => {
                            // Handle typed block results
                            if let Some(temp) = frame.block_result_local {
                                // Pop fallthrough result and assign to temp
                                let fallthrough_val = self.pop();
                                let mut body_stmts = body.stmts;
                                body_stmts.push(Stmt::LocalSet {
                                    local: temp,
                                    value: fallthrough_val,
                                });
                                self.emit(Stmt::Block {
                                    label: frame.label,
                                    body: crate::ir::Block::with_stmts(body_stmts),
                                });
                                // Push the result temp onto the stack
                                self.push(Expr::local(temp));
                            } else {
                                self.emit(Stmt::Block {
                                    label: frame.label,
                                    body,
                                });
                            }
                        }
                        ControlKind::Loop => {
                            self.emit(Stmt::Loop {
                                label: frame.label,
                                body,
                            });
                        }
                        ControlKind::If => {
                            let result_count = self.block_result_count(frame.blockty);

                            // Extract result values from the else branch (on stack now)
                            let else_results: Vec<Expr> = if result_count > 0 {
                                let drain_start =
                                    self.stack.len().saturating_sub(result_count);
                                self.stack.drain(drain_start..).collect()
                            } else {
                                Vec::new()
                            };

                            // Use saved condition from frame (not the stack)
                            let cond = frame
                                .saved_cond
                                .unwrap_or_else(|| Expr::i32_const(0));

                            // Fix branch ordering: at Else, the TRUE branch stmts
                            // were saved as frame.else_stmts, and at End, `body`
                            // contains the FALSE branch stmts. Swap them back.
                            let (then_block, else_block) = match frame.else_stmts {
                                Some(true_branch_stmts) => {
                                    // Had else: true_branch was saved, body is false branch
                                    (
                                        crate::ir::Block::with_stmts(true_branch_stmts),
                                        Some(body),
                                    )
                                }
                                None => {
                                    // No else: body is the true branch
                                    (body, None)
                                }
                            };

                            if result_count > 0
                                && frame.then_result.is_some()
                                && !else_results.is_empty()
                            {
                                let then_results = frame.then_result.unwrap();
                                // Assign result values to temp locals in each branch,
                                // rather than re-evaluating the condition with Select.
                                // The condition's operands may have been modified by
                                // side effects in the if/else body.
                                let mut then_block = then_block;
                                let mut else_block_inner = else_block
                                    .unwrap_or_else(|| crate::ir::Block::with_stmts(vec![]));
                                let mut temps = Vec::new();
                                for (then_val, else_val) in
                                    then_results.into_iter().zip(else_results.into_iter())
                                {
                                    let temp = self.next_temp_local;
                                    self.next_temp_local += 1;
                                    then_block.stmts.push(Stmt::LocalSet {
                                        local: temp,
                                        value: then_val,
                                    });
                                    else_block_inner.stmts.push(Stmt::LocalSet {
                                        local: temp,
                                        value: else_val,
                                    });
                                    temps.push(temp);
                                }
                                self.emit(Stmt::If {
                                    cond,
                                    then_block,
                                    else_block: Some(else_block_inner),
                                });
                                for temp in temps {
                                    self.push(Expr::local(temp));
                                }
                            } else {
                                self.emit(Stmt::If {
                                    cond,
                                    then_block,
                                    else_block,
                                });
                            }
                        }
                    }
                }
            }

            Br { relative_depth } => {
                let target = self.get_branch_target(relative_depth);
                // Handle typed block results: assign carried values to temp
                self.emit_block_result_assignment(relative_depth);
                self.emit(Stmt::Br {
                    label: target.label,
                    is_loop: target.is_loop,
                });
            }

            BrIf { relative_depth } => {
                let cond = self.pop();
                let target = self.get_branch_target(relative_depth);

                // Check if this branch carries values to a typed block
                let result_count = self.get_branch_result_count(relative_depth);
                if result_count > 0 && !target.is_loop {
                    // Peek (not pop) the result value — br_if only consumes it when taken;
                    // on the "not taken" path, the value must remain on the stack.
                    let result_val = self.stack.last().cloned().unwrap_or_else(|| Expr::i32_const(0));
                    let temp = self.ensure_block_result_local(relative_depth);
                    self.emit(Stmt::If {
                        cond,
                        then_block: crate::ir::Block::with_stmts(vec![
                            Stmt::LocalSet { local: temp, value: result_val },
                            Stmt::Br { label: target.label, is_loop: false },
                        ]),
                        else_block: None,
                    });
                } else {
                    self.emit(Stmt::BrIf {
                        label: target.label,
                        cond,
                        is_loop: target.is_loop,
                    });
                }
            }

            BrTable { targets } => {
                let index = self.pop();
                let target_list: Vec<BranchTarget> = targets
                    .targets()
                    .filter_map(|t| t.ok())
                    .map(|d| self.get_branch_target(d))
                    .collect();
                let default = self.get_branch_target(targets.default());
                self.emit(Stmt::BrTable {
                    index,
                    targets: target_list,
                    default,
                });
            }

            Return => {
                let value = if !self.stack.is_empty() {
                    Some(self.pop())
                } else {
                    None
                };
                self.emit(Stmt::Return(value));
            }

            Select => {
                // WASM select: pop cond, pop val2, pop val1
                // If cond != 0, result = val1, else result = val2
                let cond = self.pop();
                let (val1, val2) = self.pop2();
                self.push(Expr::new(ExprKind::Select {
                    cond: Box::new(cond),
                    then_val: Box::new(val1),
                    else_val: Box::new(val2),
                }));
            }

            Drop => {
                let val = self.pop();
                self.emit(Stmt::Drop(val));
            }

            Unreachable => {
                self.emit(Stmt::Unreachable);
            }

            Nop => {
                self.emit(Stmt::Nop);
            }

            MemorySize { .. } => {
                // Return memory size in pages
                self.push(Expr::new(ExprKind::Call {
                    func: u32::MAX, // Special marker for memory.size
                    args: vec![],
                }));
            }

            MemoryGrow { .. } => {
                let pages = self.pop();
                self.push(Expr::new(ExprKind::Call {
                    func: u32::MAX - 1, // Special marker for memory.grow
                    args: vec![pages],
                }));
            }

            // Bulk memory operations
            MemoryFill { .. } => {
                // memory.fill(dest, value, len) - fills memory with a byte value
                let len = self.pop();
                let value = self.pop();
                let dest = self.pop();
                self.emit(Stmt::Expr(Expr::new(ExprKind::Call {
                    func: u32::MAX - 2, // Special marker for memory.fill
                    args: vec![dest, value, len],
                })));
            }

            MemoryCopy { .. } => {
                // memory.copy(dest, src, len) - copies memory regions
                let len = self.pop();
                let src = self.pop();
                let dest = self.pop();
                self.emit(Stmt::Expr(Expr::new(ExprKind::Call {
                    func: u32::MAX - 3, // Special marker for memory.copy
                    args: vec![dest, src, len],
                })));
            }

            // Default: treat as nop
            _ => {}
        }

        Ok(())
    }

    fn lift_load(&mut self, offset: u32, size: MemSize, signed: bool) {
        let addr = self.pop();
        let ty = match size {
            MemSize::I8 | MemSize::I16 | MemSize::I32 => InferredType::I32,
            MemSize::I64 => InferredType::I64,
            MemSize::F32 => InferredType::F32,
            MemSize::F64 => InferredType::F64,
        };
        self.push(Expr::with_type(
            ExprKind::Load {
                addr: Box::new(addr),
                offset,
                size,
                signed,
            },
            ty,
        ));
    }

    fn lift_store(&mut self, offset: u32, size: MemSize) {
        let value = self.pop();
        let addr = self.pop();
        self.emit(Stmt::Store {
            addr,
            offset,
            value,
            size,
        });
    }

    fn lift_binop(&mut self, op: BinOp, ty: InferredType) {
        let (a, b) = self.pop2();
        self.push(Expr::with_type(
            ExprKind::BinOp(op, Box::new(a), Box::new(b)),
            ty,
        ));
    }

    fn lift_unaryop(&mut self, op: UnaryOp, ty: InferredType) {
        let a = self.pop();
        self.push(Expr::with_type(ExprKind::UnaryOp(op, Box::new(a)), ty));
    }

    fn lift_cmpop(&mut self, op: CmpOp, operand_ty: InferredType) {
        let (a, b) = self.pop2();
        self.push(Expr::with_type(
            ExprKind::Compare(op, Box::new(a), Box::new(b), operand_ty),
            InferredType::Bool,
        ));
    }

    fn lift_convert(&mut self, op: ConvertOp, ty: InferredType) {
        let a = self.pop();
        self.push(Expr::with_type(
            ExprKind::Convert {
                op,
                expr: Box::new(a),
            },
            ty,
        ));
    }

    /// Get result count for a branch target (0 for loops, block result count otherwise)
    fn get_branch_result_count(&self, relative_depth: u32) -> usize {
        let idx = self
            .control_stack
            .len()
            .saturating_sub(1 + relative_depth as usize);
        self.control_stack
            .get(idx)
            .map(|f| {
                if matches!(f.kind, ControlKind::Loop) {
                    0
                } else {
                    self.block_result_count(f.blockty)
                }
            })
            .unwrap_or(0)
    }

    /// Ensure the target block has a result temp local allocated, return it
    fn ensure_block_result_local(&mut self, relative_depth: u32) -> u32 {
        let idx = self
            .control_stack
            .len()
            .saturating_sub(1 + relative_depth as usize);
        if let Some(frame) = self.control_stack.get(idx) {
            if let Some(local) = frame.block_result_local {
                return local;
            }
        }
        let temp = self.next_temp_local;
        self.next_temp_local += 1;
        if let Some(frame) = self.control_stack.get_mut(idx) {
            frame.block_result_local = Some(temp);
        }
        temp
    }

    /// Emit assignment of stack values to block result temp for Br to typed blocks
    fn emit_block_result_assignment(&mut self, relative_depth: u32) {
        let result_count = self.get_branch_result_count(relative_depth);
        if result_count > 0 {
            let result_val = self.pop();
            let temp = self.ensure_block_result_local(relative_depth);
            self.emit(Stmt::LocalSet {
                local: temp,
                value: result_val,
            });
        }
    }

    fn get_branch_target(&self, relative_depth: u32) -> BranchTarget {
        let idx = self
            .control_stack
            .len()
            .saturating_sub(1 + relative_depth as usize);
        self.control_stack
            .get(idx)
            .map(|f| BranchTarget {
                label: f.label,
                is_loop: matches!(f.kind, ControlKind::Loop),
            })
            .unwrap_or(BranchTarget {
                label: 0,
                is_loop: false,
            })
    }

    fn finish(mut self) -> Block {
        // Any remaining stack values should be returned
        if !self.stack.is_empty() {
            let val = self.pop();
            self.emit(Stmt::Return(Some(val)));
        }
        Block::with_stmts(self.stmts)
    }
}
