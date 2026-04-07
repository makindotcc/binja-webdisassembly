//! WASM to JavaScript Transpiler CLI
//!
//! Usage:
//!   wasm-decompile input.wasm -o output.js
//!   wasm-decompile input.wasm -o output.js --target go
//!   wasm-decompile input.wasm --dump-ir

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};

use wasm_decompile::{decompile, decompile_to_ir, dump_ir, DecompileOptions, Target};

#[derive(Parser, Debug)]
#[command(name = "wasm-decompile")]
#[command(author, version, about = "WASM to JavaScript transpiler with multi-pass architecture")]
struct Args {
    /// Input WASM file
    #[arg(required = true)]
    input: PathBuf,

    /// Output JavaScript file (stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Target compiler/runtime for specialized passes
    #[arg(short, long, value_enum, default_value = "generic")]
    target: TargetArg,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Dump IR instead of generating JavaScript
    #[arg(long)]
    dump_ir: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TargetArg {
    Generic,
    Go,
    Rust,
    C,
}

impl From<TargetArg> for Target {
    fn from(arg: TargetArg) -> Self {
        match arg {
            TargetArg::Generic => Target::Generic,
            TargetArg::Go => Target::Go,
            TargetArg::Rust => Target::Rust,
            TargetArg::C => Target::C,
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Read input file
    let wasm_bytes = fs::read(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input.display()))?;

    // Set up options
    let options = DecompileOptions::new()
        .with_target(args.target.into())
        .with_debug(args.verbose)
        .with_dump_ir(args.dump_ir);

    if args.verbose {
        eprintln!("Input: {}", args.input.display());
        eprintln!("Target: {:?}", args.target);
        eprintln!("WASM size: {} bytes", wasm_bytes.len());
    }

    // Decompile
    let output = if args.dump_ir {
        let module = decompile_to_ir(&wasm_bytes, &options)
            .context("Failed to decompile WASM")?;
        dump_ir(&module)
    } else {
        decompile(&wasm_bytes, &options)
            .context("Failed to decompile WASM")?
    };

    // Write output
    if let Some(output_path) = &args.output {
        fs::write(output_path, &output)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
        if args.verbose {
            eprintln!("Output written to: {}", output_path.display());
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}
