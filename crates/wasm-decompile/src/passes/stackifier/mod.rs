//! Stackifier pass for structured control flow recovery
//!
//! Uses CFG analysis to emit structured code from WASM block/loop/br patterns.

mod emit;
mod transform;

use crate::ir::Module;
use crate::passes::{Pass, PassContext};

use emit::emit_structured;
use transform::transform_function;

/// Stackifier pass that transforms block/br patterns to structured code
pub struct StackifierPass;

impl Pass for StackifierPass {
    fn name(&self) -> &'static str {
        "stackifier"
    }

    fn run(&self, module: &mut Module, ctx: &mut PassContext) {
        for func in &mut module.functions {
            if func.is_import {
                continue;
            }

            ctx.log(format!(
                "Stackifier: processing function {}",
                func.name.as_deref().unwrap_or("(anonymous)")
            ));

            // Phase 1: CFG-based structured emission
            func.body = emit_structured(&func.body);

            // Phase 2: Direct transforms for remaining patterns
            transform_function(&mut func.body, ctx);
        }
    }
}
