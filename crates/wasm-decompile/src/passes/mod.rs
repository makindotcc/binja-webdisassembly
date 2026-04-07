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

use crate::ir::{InferredType, Module, RuntimeHelperDecl};
use std::collections::HashMap;

/// Identifies which emitter a helper implementation targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmitterId(pub &'static str);

/// Registry for runtime helper implementations keyed by (helper, emitter)
pub struct HelperRegistry {
    impls: HashMap<(RuntimeHelperDecl, EmitterId), String>,
}

impl HelperRegistry {
    pub fn new() -> Self {
        Self {
            impls: HashMap::new(),
        }
    }

    /// Register a helper implementation for a specific emitter
    pub fn register(&mut self, helper: RuntimeHelperDecl, emitter: EmitterId, code: String) {
        self.impls.insert((helper, emitter), code);
    }

    /// Get a helper implementation for a specific emitter
    pub fn get(&self, helper: RuntimeHelperDecl, emitter: EmitterId) -> Option<&str> {
        self.impls.get(&(helper, emitter)).map(|s| s.as_str())
    }

    /// Iterate over all helpers for a specific emitter
    pub fn iter_for(
        &self,
        emitter: EmitterId,
    ) -> impl Iterator<Item = (RuntimeHelperDecl, &String)> + '_ {
        self.impls
            .iter()
            .filter(move |((_, e), _)| *e == emitter)
            .map(|((h, _), code)| (*h, code))
    }
}

impl Default for HelperRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A transformation pass on the IR
pub trait Pass {
    /// Name of the pass (for logging)
    fn name(&self) -> &'static str;

    /// Transform the module in-place
    fn run(&self, module: &mut Module, ctx: &mut PassContext);

    /// Register runtime helper implementations for target languages.
    /// Called by the pipeline before emission.
    fn register_helpers(&self, _registry: &mut HelperRegistry) {}
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
                Box::new(control_flow::ControlFlowPass),
                Box::new(mem_resolve::MemResolvePass),
                // Box::new(stackifier::StackifierPass),
                // Box::new(UnblockifyPass),
                // Box::new(type_infer::TypeInferPass),
                // Box::new(go::GoSlicePass),
            ],
        }
    }

    /// Create pipeline optimized for Go-compiled WASM
    pub fn for_go() -> Self {
        Self {
            passes: vec![
                Box::new(go::AsyncifyPass),
                Box::new(go::GoDeferPass),
                Box::new(control_flow::ControlFlowPass),
                Box::new(mem_resolve::MemResolvePass),
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

    /// Collect all runtime helper implementations from passes
    pub fn collect_helpers(&self) -> HelperRegistry {
        let mut registry = HelperRegistry::new();
        for pass in &self.passes {
            pass.register_helpers(&mut registry);
        }
        registry
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::default_pipeline()
    }
}
