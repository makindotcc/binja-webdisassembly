# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a WebAssembly plugin for Binary Ninja, written in Rust. It provides:
1. **binja-wasm**: A Binary Ninja plugin for disassembly and decompilation of WASM binaries
2. **wasm-decompile**: A standalone WASM-to-JavaScript transpiler with multi-pass architecture
3. **example-wasm**: Sample WASM functions for testing

## Build Commands

```bash
# Build the Binary Ninja plugin (requires Binary Ninja installation)
cargo build -p binja-wasm --release

# Build the standalone decompiler
cargo build -p wasm-decompile --release

# Build example WASM module
cargo build -p example-wasm --target wasm32-unknown-unknown --release

# Run tests
cargo test

# Run standalone decompiler
cargo run -p wasm-decompile -- <input.wasm>
```

The binja-wasm crate requires Binary Ninja to be installed. The build script links against `libbinaryninjacore`.

## Architecture

### binja-wasm (Binary Ninja Plugin)

The plugin implements Binary Ninja's architecture and custom binary view interfaces:

- **lib.rs**: Plugin entry point, registers architecture, platform, calling convention, and view type via `CorePluginInit()`
- **arch.rs**: WASM architecture implementation with:
  - `WasmArchitecture`: Implements `Architecture` trait for instruction decoding, LLIL lifting, and basic block analysis
  - `WasmRegister`: Virtual registers (sp, ret, arg0-arg7) mapping WASM's stack machine to register-based IL
  - `WasmCallingConvention`: Calling convention with arg0-arg7 for parameters, ret for return values
  - `StackState`: Tracks WASM value stack depth during LLIL lifting using temp registers
- **view.rs**: Custom binary view (`WasmView`) that:
  - Parses WASM modules and creates segments for Code, Data, Globals, and LinearMemory
  - Maps WASM globals to virtual addresses at `0x8000_0000`
  - Maps linear memory to virtual addresses at `0x1000_0000`
  - Defines Go/TinyGo types (GoString, GoSlice, GoInterface)
- **decode.rs**: Instruction decoder using wasmparser, classifies instructions by kind (control flow, arithmetic, memory, etc.)
- **analysis.rs**: WASM-specific analysis including:
  - Block/branch target resolution for structured control flow
  - Stack depth tracking per instruction for LLIL temp register mapping
  - `ANALYZED_MODULES`: Global registry keyed by BinaryView handle
- **wasm.rs**: WASM module parser extracting functions, globals, memory, data segments

### wasm-decompile (Standalone Transpiler)

Multi-pass decompiler: WASM bytecode → IR → (passes) → JavaScript

- **lift.rs**: WASM bytecode to IR transformation
- **ir.rs**: Intermediate representation types (Module, Function, Block, Stmt, Expr)
- **passes/**: Transformation passes:
  - `type_infer.rs`: Type inference
  - `simplify.rs`: Constant folding, dead code elimination
  - `mem_resolve.rs`: String literal resolution from memory
  - `control_flow.rs`: Control flow restructuring
  - `go/`: Go-specific patterns (strings, slices, interfaces)
- **emit_js.rs**: JavaScript code generation

## Key Constants

- `GLOBALS_BASE_ADDR = 0x8000_0000`: Virtual segment for WASM globals
- `LINEAR_MEMORY_BASE = 0x1000_0000`: Virtual segment for WASM linear memory
- `STACK_TEMP_BASE = 0xFFFF`: Base offset for stack temp registers in LLIL lifting

## WASM Stack Machine Mapping

WASM is a stack machine; Binary Ninja uses registers. The plugin maps:
- Function parameters → arg0-arg7 registers, then to local temp registers at function entry
- Local variables → temp registers (temp0, temp1, ...)
- Value stack → temp registers starting at `STACK_TEMP_BASE + depth`
- Return values → ret register
