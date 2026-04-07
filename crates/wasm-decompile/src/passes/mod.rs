//! Transformation passes for the IR
//!
//! Passes transform the IR to improve readability and add high-level constructs.

pub mod control_flow;
pub mod go;
pub mod mem_resolve;
pub mod simplify;
pub mod stackifier;
pub mod type_infer;
pub mod unblockify;

use std::collections::HashMap;

use crate::{
    ir::{InferredType, Module},
    passes::unblockify::UnblockifyPass,
};

/// A transformation pass on the IR
pub trait Pass {
    /// Name of the pass (for logging)
    fn name(&self) -> &'static str;

    /// Transform the module in-place
    fn run(&self, module: &mut Module, ctx: &mut PassContext);
}

/// Context shared between passes
pub struct PassContext {
    /// Inferred types for variables (function_idx, local_idx) -> type
    pub var_types: HashMap<(u32, u32), InferredType>,

    /// Known function signatures/purposes
    pub known_functions: HashMap<u32, KnownFunction>,

    /// Diagnostics and warnings
    pub diagnostics: Vec<Diagnostic>,

    /// Whether to emit debug info
    pub debug: bool,
}

impl PassContext {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            known_functions: HashMap::new(),
            diagnostics: Vec::new(),
            debug: false,
        }
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn log(&mut self, message: String) {
        if self.debug {
            self.diagnostics.push(Diagnostic::Info(message));
        }
    }

    pub fn warn(&mut self, message: String) {
        self.diagnostics.push(Diagnostic::Warning(message));
    }
}

impl Default for PassContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Known function types for pattern recognition
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownFunction {
    // Memory allocation
    Malloc,
    Free,
    Realloc,
    MemCopy,
    MemSet,

    // String operations
    StringConcat,
    StringLen,
    StringCmp,

    // Go runtime
    GoRuntimeMalloc,
    GoRuntimeSliceAppend,
    GoRuntimeStringConcat,
    GoPanic,
    GoMakeSlice,
    GoMakeMap,
    GoMakeChan,

    // Rust runtime
    RustPanic,
    RustAlloc,
    RustDealloc,

    // I/O
    Print,
    PrintLn,
    Read,
    Write,

    // Custom/user-defined
    Custom(String),
}

/// Diagnostic message from a pass
#[derive(Debug, Clone)]
pub enum Diagnostic {
    Info(String),
    Warning(String),
    Error(String),
}

/// Pipeline of passes
pub struct Pipeline {
    passes: Vec<Box<dyn Pass>>,
}

impl Pipeline {
    /// Create an empty pipeline
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Create default pipeline with basic passes
    pub fn default_pipeline() -> Self {
        Self {
            passes: vec![
                // Box::new(simplify::SimplifyPass),
                // Box::new(control_flow::ControlFlowPass),
                // Box::new(stackifier::StackifierPass),
                // Box::new(UnblockifyPass),
                // Box::new(type_infer::TypeInferPass),
                // Box::new(mem_resolve::MemResolvePass),
            ],
        }
    }

    /// Create pipeline optimized for Go-compiled WASM
    pub fn for_go() -> Self {
        Self {
            passes: vec![
                Box::new(simplify::SimplifyPass),
                Box::new(control_flow::ControlFlowPass),
                Box::new(type_infer::TypeInferPass),
                Box::new(go::GoStringPass),
                Box::new(go::GoSlicePass),
                Box::new(mem_resolve::MemResolvePass),
                Box::new(simplify::SimplifyPass), // Run again after Go passes
            ],
        }
    }

    /// Create pipeline for Rust-compiled WASM
    pub fn for_rust() -> Self {
        Self {
            passes: vec![
                Box::new(simplify::SimplifyPass),
                Box::new(control_flow::ControlFlowPass),
                Box::new(type_infer::TypeInferPass),
                Box::new(mem_resolve::MemResolvePass),
            ],
        }
    }

    /// Add a pass to the pipeline
    pub fn add_pass(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass);
    }

    /// Run all passes on the module
    pub fn run(&self, module: &mut Module, ctx: &mut PassContext) {
        for pass in &self.passes {
            ctx.log(format!("Running pass: {}", pass.name()));
            pass.run(module, ctx);
        }
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::default_pipeline()
    }
}
