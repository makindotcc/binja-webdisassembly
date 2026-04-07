//! WASM to JavaScript Transpiler
//!
//! A multi-pass decompiler that transforms WebAssembly bytecode to readable JavaScript.
//!
//! # Architecture
//!
//! ```text
//! WASM bytecode
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  1. Lifting       │  WASM → Raw IR
//! └─────────────────┘
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  2. Passes      │  Transformacje IR → IR
//! │  ┌───────────┐  │
//! │  │ TypeInfer  │  │  - type inference
//! │  ├───────────┤  │
//! │  │ GoPatterns │  │  - string, slice, interface
//! │  ├───────────┤  │
//! │  │ Simplify   │  │  - const folding, dead code
//! │  ├───────────┤  │
//! │  │ MemResolve │  │  - string literals from memory
//! │  └───────────┘  │
//! └─────────────────┘
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  3. Emit          │  IR → JavaScript
//! └─────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use wasm_decompile::{decompile, DecompileOptions, Target};
//!
//! let wasm_bytes = std::fs::read("example.wasm").unwrap();
//! let options = DecompileOptions::default();
//! let js_code = decompile(&wasm_bytes, &options).unwrap();
//! println!("{}", js_code);
//! ```

pub mod cfg;
pub mod emit_js;
pub mod ir;
pub mod lift;
pub mod passes;

use anyhow::Result;

pub use emit_js::JsEmitter;
pub use ir::*;
pub use lift::lift;
pub use passes::{Pass, PassContext, Pipeline};

/// Target compiler/runtime for specialized passes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Target {
    /// Generic WASM (default passes only)
    #[default]
    Generic,
    /// Go-compiled WASM
    Go,
    /// Rust-compiled WASM
    Rust,
    /// C/C++-compiled WASM (Emscripten)
    C,
}

/// Options for decompilation
#[derive(Debug, Clone, Default)]
pub struct DecompileOptions {
    /// Target compiler/runtime
    pub target: Target,
    /// Enable debug output
    pub debug: bool,
    /// Dump IR after each pass
    pub dump_ir: bool,
}

impl DecompileOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn with_dump_ir(mut self, dump_ir: bool) -> Self {
        self.dump_ir = dump_ir;
        self
    }
}

/// Decompile WASM bytecode to JavaScript
pub fn decompile(wasm: &[u8], options: &DecompileOptions) -> Result<String> {
    // Step 1: Lift WASM to IR
    let mut module = lift(wasm)?;

    // Step 2: Select and run passes
    let pipeline = match options.target {
        Target::Generic => Pipeline::default_pipeline(),
        Target::Go => Pipeline::for_go(),
        Target::Rust => Pipeline::for_rust(),
        Target::C => Pipeline::default_pipeline(), // TODO: Add C-specific passes
    };

    let mut ctx = PassContext::new().with_debug(options.debug);
    pipeline.run(&mut module, &mut ctx);

    // Print diagnostics if debug enabled
    if options.debug {
        for diag in &ctx.diagnostics {
            match diag {
                passes::Diagnostic::Info(msg) => eprintln!("[INFO] {}", msg),
                passes::Diagnostic::Warning(msg) => eprintln!("[WARN] {}", msg),
                passes::Diagnostic::Error(msg) => eprintln!("[ERROR] {}", msg),
            }
        }
    }

    // Step 3: Emit JavaScript
    let mut emitter = JsEmitter::new();
    let js = emitter.emit(&module);

    Ok(js)
}

/// Decompile and return the IR (for debugging/analysis)
pub fn decompile_to_ir(wasm: &[u8], options: &DecompileOptions) -> Result<Module> {
    let mut module = lift(wasm)?;

    let pipeline = match options.target {
        Target::Generic => Pipeline::default_pipeline(),
        Target::Go => Pipeline::for_go(),
        Target::Rust => Pipeline::for_rust(),
        Target::C => Pipeline::default_pipeline(),
    };

    let mut ctx = PassContext::new().with_debug(options.debug);
    pipeline.run(&mut module, &mut ctx);

    Ok(module)
}

/// Dump IR for debugging
pub fn dump_ir(module: &Module) -> String {
    let mut output = String::new();

    output.push_str("=== Module IR ===\n\n");

    // Dump imports
    if !module.imports.is_empty() {
        output.push_str("-- Imports --\n");
        for import in &module.imports {
            output.push_str(&format!(
                "  import {}.{}: {:?}\n",
                import.module, import.name, import.kind
            ));
        }
        output.push('\n');
    }

    // Dump globals
    if !module.globals.is_empty() {
        output.push_str("-- Globals --\n");
        for (i, global) in module.globals.iter().enumerate() {
            let mutability = if global.mutable { "mut" } else { "const" };
            output.push_str(&format!("  g{}: {:?} {}\n", i, global.ty, mutability));
        }
        output.push('\n');
    }

    // Dump functions
    output.push_str("-- Functions --\n");
    for func in &module.functions {
        let name = func.name.as_deref().unwrap_or("(anonymous)");
        let import_marker = if func.is_import { " [import]" } else { "" };
        output.push_str(&format!(
            "\nfunc {} (idx={}){}:\n",
            name, func.index, import_marker
        ));

        if !func.is_import {
            output.push_str(&format!("  params: {:?}\n", func.params));
            output.push_str(&format!("  results: {:?}\n", func.results));
            output.push_str(&format!("  locals: {:?}\n", func.locals));
            output.push_str("  body:\n");
            dump_block(&func.body, 2, &mut output);
        }
    }

    output
}

fn dump_block(block: &Block, indent: usize, output: &mut String) {
    for stmt in &block.stmts {
        dump_stmt(stmt, indent, output);
    }
}

fn dump_stmt(stmt: &Stmt, indent: usize, output: &mut String) {
    let prefix = "  ".repeat(indent);
    match stmt {
        Stmt::LocalSet { local, value } => {
            output.push_str(&format!("{}local.set {} = ", prefix, local));
            dump_expr(value, output);
            output.push('\n');
        }
        Stmt::GlobalSet { global, value } => {
            output.push_str(&format!("{}global.set {} = ", prefix, global));
            dump_expr(value, output);
            output.push('\n');
        }
        Stmt::Store {
            addr, value, size, ..
        } => {
            output.push_str(&format!("{}{:?}.store ", prefix, size));
            dump_expr(addr, output);
            output.push_str(" = ");
            dump_expr(value, output);
            output.push('\n');
        }
        Stmt::Expr(expr) => {
            output.push_str(&prefix);
            dump_expr(expr, output);
            output.push('\n');
        }
        Stmt::Return(val) => {
            output.push_str(&format!("{}return", prefix));
            if let Some(v) = val {
                output.push(' ');
                dump_expr(v, output);
            }
            output.push('\n');
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            output.push_str(&format!("{}if ", prefix));
            dump_expr(cond, output);
            output.push_str(" {\n");
            dump_block(then_block, indent + 1, output);
            if let Some(else_blk) = else_block {
                output.push_str(&format!("{}}} else {{\n", prefix));
                dump_block(else_blk, indent + 1, output);
            }
            output.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::Block { label, body } => {
            output.push_str(&format!("{}block_{}: {{\n", prefix, label));
            dump_block(body, indent + 1, output);
            output.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::Loop { label, body } => {
            output.push_str(&format!("{}loop_{}: {{\n", prefix, label));
            dump_block(body, indent + 1, output);
            output.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::DoWhile { body, cond } => {
            output.push_str(&format!("{}do {{\n", prefix));
            dump_block(body, indent + 1, output);
            output.push_str(&format!("{}}} while ", prefix));
            dump_expr(cond, output);
            output.push('\n');
        }
        Stmt::While { cond, body } => {
            output.push_str(&format!("{}while ", prefix));
            dump_expr(cond, output);
            output.push_str(" {\n");
            dump_block(body, indent + 1, output);
            output.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::Br { label, is_loop } => {
            let prefix_name = if *is_loop { "loop" } else { "block" };
            output.push_str(&format!("{}br {}_{}\n", prefix, prefix_name, label));
        }
        Stmt::BrIf {
            label,
            cond,
            is_loop,
        } => {
            let prefix_name = if *is_loop { "loop" } else { "block" };
            output.push_str(&format!("{}br_if {}_{} ", prefix, prefix_name, label));
            dump_expr(cond, output);
            output.push('\n');
        }
        Stmt::BrTable {
            index,
            targets,
            default,
        } => {
            output.push_str(&format!("{}br_table ", prefix));
            dump_expr(index, output);
            let targets_str: Vec<String> = targets
                .iter()
                .map(|t| {
                    let p = if t.is_loop { "loop" } else { "block" };
                    format!("{}_{}", p, t.label)
                })
                .collect();
            let default_p = if default.is_loop { "loop" } else { "block" };
            output.push_str(&format!(
                " [{}] default={}_{}\n",
                targets_str.join(", "),
                default_p,
                default.label
            ));
        }
        Stmt::Unreachable => {
            output.push_str(&format!("{}unreachable\n", prefix));
        }
        Stmt::Nop => {
            output.push_str(&format!("{}nop\n", prefix));
        }
        Stmt::Drop(expr) => {
            output.push_str(&format!("{}drop ", prefix));
            dump_expr(expr, output);
            output.push('\n');
        }
    }
}

fn dump_expr(expr: &Expr, output: &mut String) {
    match &expr.kind {
        ExprKind::I32Const(v) => output.push_str(&format!("i32({})", v)),
        ExprKind::I64Const(v) => output.push_str(&format!("i64({})", v)),
        ExprKind::F32Const(v) => output.push_str(&format!("f32({})", v)),
        ExprKind::F64Const(v) => output.push_str(&format!("f64({})", v)),
        ExprKind::Local(idx) => output.push_str(&format!("local.{}", idx)),
        ExprKind::Global(idx) => output.push_str(&format!("global.{}", idx)),
        ExprKind::BinOp(op, a, b) => {
            output.push('(');
            dump_expr(a, output);
            output.push_str(&format!(" {:?} ", op));
            dump_expr(b, output);
            output.push(')');
        }
        ExprKind::UnaryOp(op, a) => {
            output.push_str(&format!("{:?}(", op));
            dump_expr(a, output);
            output.push(')');
        }
        ExprKind::Compare(op, a, b) => {
            output.push('(');
            dump_expr(a, output);
            output.push_str(&format!(" {:?} ", op));
            dump_expr(b, output);
            output.push(')');
        }
        ExprKind::Load {
            addr, offset, size, ..
        } => {
            output.push_str(&format!("{:?}.load[", size));
            dump_expr(addr, output);
            if *offset > 0 {
                output.push_str(&format!("+{}", offset));
            }
            output.push(']');
        }
        ExprKind::Call { func, args } => {
            output.push_str(&format!("call {}(", func));
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                dump_expr(arg, output);
            }
            output.push(')');
        }
        ExprKind::CallIndirect {
            type_idx,
            index,
            args,
            ..
        } => {
            output.push_str(&format!("call_indirect[type={}](", type_idx));
            dump_expr(index, output);
            for arg in args {
                output.push_str(", ");
                dump_expr(arg, output);
            }
            output.push(')');
        }
        ExprKind::Select {
            cond,
            then_val,
            else_val,
        } => {
            output.push_str("select(");
            dump_expr(cond, output);
            output.push_str(", ");
            dump_expr(then_val, output);
            output.push_str(", ");
            dump_expr(else_val, output);
            output.push(')');
        }
        ExprKind::Convert { op, expr } => {
            output.push_str(&format!("{:?}(", op));
            dump_expr(expr, output);
            output.push(')');
        }
        ExprKind::StringLiteral(s) => {
            output.push_str(&format!("\"{}\"", s.escape_default()));
        }
        ExprKind::GoString { ptr, len } => {
            output.push_str("go_string(");
            dump_expr(ptr, output);
            output.push_str(", ");
            dump_expr(len, output);
            output.push(')');
        }
        ExprKind::GoSlice { ptr, len, cap } => {
            output.push_str("go_slice(");
            dump_expr(ptr, output);
            output.push_str(", ");
            dump_expr(len, output);
            output.push_str(", ");
            dump_expr(cap, output);
            output.push(')');
        }
        ExprKind::GoInterface { type_ptr, data } => {
            output.push_str("go_interface(");
            dump_expr(type_ptr, output);
            output.push_str(", ");
            dump_expr(data, output);
            output.push(')');
        }
        ExprKind::ResolvedPointer { addr, resolved } => {
            output.push_str(&format!(
                "resolved[0x{:x}]=\"{}\"",
                addr,
                resolved.escape_default()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_module() {
        // Minimal valid WASM module
        let wasm = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version
        ];

        let result = decompile(&wasm, &DecompileOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_target_selection() {
        let options = DecompileOptions::new().with_target(Target::Go);
        assert_eq!(options.target, Target::Go);
    }
}
