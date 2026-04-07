//! WASM module parsing using wasmparser

use eyre::Context;
use wasmparser::{Name, NameSectionReader, Parser, Payload};

#[derive(Debug, Clone)]
pub struct WasmModule {
    pub version: u32,
    pub sections: Vec<WasmSection>,
    pub functions: Vec<WasmFunction>,
    pub globals: Vec<WasmGlobal>,
    pub start_function: Option<u32>,
    pub _exports: Vec<WasmExport>,
    /// Linear memory limits (min pages, max pages). Each page is 64KB.
    pub memory: Option<WasmMemory>,
    /// Data segments that initialize linear memory.
    pub data_segments: Vec<WasmDataSegment>,
}

#[derive(Debug, Clone)]
pub struct WasmMemory {
    /// Minimum number of 64KB pages.
    pub initial_pages: u32,
    /// Maximum number of 64KB pages (if specified).
    pub max_pages: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct WasmDataSegment {
    /// Offset in linear memory where this segment starts.
    pub memory_offset: u32,
    /// The data bytes.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum WasmSection {
    Code {
        offset: usize,
        size: usize,
    },
    Data {
        offset: usize,
        size: usize,
    },
    Other {
        name: String,
        offset: usize,
        size: usize,
    },
}

#[derive(Debug, Clone)]
pub struct WasmFunction {
    pub index: u32,
    pub code_offset: usize,
    pub code_size: usize,
    pub name: Option<String>,
    pub param_count: usize,
    pub return_count: usize,
}

#[derive(Debug, Clone)]
pub struct WasmExport {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct WasmGlobal {
    pub index: u32,
    pub name: Option<String>,
    pub val_type: GlobalValType,
    pub mutable: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum GlobalValType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone, Copy)]
pub enum ExportKind {
    Func,
    Table,
    Memory,
    Global,
}

impl WasmModule {
    pub fn parse(data: &[u8]) -> eyre::Result<Self> {
        let parser = Parser::new(0);

        let mut version = 1;
        let mut sections = Vec::new();
        let mut functions = Vec::new();
        let mut globals = Vec::new();
        let mut start_function = None;
        let mut _exports = Vec::new();
        let mut memory = None;
        let mut data_segments = Vec::new();
        let mut func_index = 0u32;
        let mut global_index = 0u32;
        let mut import_func_count = 0u32;
        let mut import_global_count = 0u32;
        let mut func_names: Vec<(u32, String)> = Vec::new();

        let mut func_types: Vec<(usize, usize)> = Vec::new();
        let mut func_type_indices: Vec<u32> = Vec::new();

        for payload in parser.parse_all(data) {
            let payload = payload.context("parse payload")?;

            match payload {
                Payload::Version { num, .. } => {
                    version = num as u32;
                }
                Payload::TypeSection(reader) => {
                    for rec_group in reader {
                        let rec_group = rec_group.context("parse type")?;
                        for sub_type in rec_group.into_types() {
                            if let wasmparser::CompositeInnerType::Func(func_type) =
                                sub_type.composite_type.inner
                            {
                                func_types
                                    .push((func_type.params().len(), func_type.results().len()));
                            }
                        }
                    }
                }
                Payload::FunctionSection(reader) => {
                    for type_idx in reader {
                        let type_idx = type_idx.context("parse function type index")?;
                        func_type_indices.push(type_idx);
                    }
                }
                Payload::ImportSection(reader) => {
                    for import in reader {
                        let import = import.context("parse import")?;
                        match import.ty {
                            wasmparser::TypeRef::Func(type_idx) => {
                                // Add imported function with code_offset=0
                                let (param_count, return_count) =
                                    func_types.get(type_idx as usize).copied().unwrap_or((0, 0));
                                functions.push(WasmFunction {
                                    index: import_func_count,
                                    code_offset: 0,
                                    code_size: 0,
                                    name: Some(format!("{}_{}", import.module, import.name)),
                                    param_count,
                                    return_count,
                                });
                                import_func_count += 1;
                            }
                            wasmparser::TypeRef::Global(gt) => {
                                let val_type = match gt.content_type {
                                    wasmparser::ValType::I32 => GlobalValType::I32,
                                    wasmparser::ValType::I64 => GlobalValType::I64,
                                    wasmparser::ValType::F32 => GlobalValType::F32,
                                    wasmparser::ValType::F64 => GlobalValType::F64,
                                    _ => GlobalValType::I32,
                                };
                                globals.push(WasmGlobal {
                                    index: import_global_count,
                                    name: Some(format!("{}_{}", import.module, import.name)),
                                    val_type,
                                    mutable: gt.mutable,
                                });
                                import_global_count += 1;
                            }
                            wasmparser::TypeRef::Memory(mem_type) => {
                                memory = Some(WasmMemory {
                                    initial_pages: mem_type.initial as u32,
                                    max_pages: mem_type.maximum.map(|m| m as u32),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                Payload::MemorySection(reader) => {
                    for mem in reader {
                        let mem = mem.context("parse memory")?;
                        memory = Some(WasmMemory {
                            initial_pages: mem.initial as u32,
                            max_pages: mem.maximum.map(|m| m as u32),
                        });
                        break; // WASM 1.0 only supports one memory
                    }
                }
                Payload::GlobalSection(reader) => {
                    for global in reader {
                        let global = global.context("parse global")?;
                        let val_type = match global.ty.content_type {
                            wasmparser::ValType::I32 => GlobalValType::I32,
                            wasmparser::ValType::I64 => GlobalValType::I64,
                            wasmparser::ValType::F32 => GlobalValType::F32,
                            wasmparser::ValType::F64 => GlobalValType::F64,
                            _ => GlobalValType::I32,
                        };
                        globals.push(WasmGlobal {
                            index: import_global_count + global_index,
                            name: None,
                            val_type,
                            mutable: global.ty.mutable,
                        });
                        global_index += 1;
                    }
                }
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export.context("parse export")?;
                        let kind = match export.kind {
                            wasmparser::ExternalKind::Func => ExportKind::Func,
                            wasmparser::ExternalKind::Table => ExportKind::Table,
                            wasmparser::ExternalKind::Memory => ExportKind::Memory,
                            wasmparser::ExternalKind::Global => ExportKind::Global,
                            _ => continue,
                        };
                        _exports.push(WasmExport {
                            name: export.name.to_string(),
                            kind,
                            index: export.index,
                        });
                    }
                }
                Payload::StartSection { func, .. } => {
                    start_function = Some(func);
                }
                Payload::CodeSectionStart { range, .. } => {
                    sections.push(WasmSection::Code {
                        offset: range.start,
                        size: range.len(),
                    });
                }
                Payload::CodeSectionEntry(body) => {
                    // Get the reader to skip past locals to actual instructions
                    let mut reader = body.get_binary_reader();
                    let _func_body_start = reader.original_position();

                    // Skip local declarations
                    let local_count = reader.read_var_u32().unwrap_or(0);
                    for _ in 0..local_count {
                        let _ = reader.read_var_u32(); // count
                        let _ = reader.read::<wasmparser::ValType>(); // type
                    }

                    let code_start = reader.original_position();
                    let range = body.range();
                    let code_size = range.end - code_start;

                    let (param_count, return_count) = func_type_indices
                        .get(func_index as usize)
                        .and_then(|&type_idx| func_types.get(type_idx as usize))
                        .copied()
                        .unwrap_or((0, 0));

                    functions.push(WasmFunction {
                        index: import_func_count + func_index,
                        code_offset: code_start,
                        code_size,
                        name: None,
                        param_count,
                        return_count,
                    });
                    func_index += 1;
                }
                Payload::DataSection(reader) => {
                    let range = reader.range();
                    sections.push(WasmSection::Data {
                        offset: range.start,
                        size: range.len(),
                    });

                    // Parse data segment entries
                    for data in reader.clone() {
                        if let Ok(data) = data {
                            // Try to extract the memory offset from the init expression
                            if let wasmparser::DataKind::Active {
                                memory_index: 0,
                                offset_expr,
                            } = data.kind
                            {
                                // Parse the const init expression to get the offset
                                let mut reader = offset_expr.get_binary_reader();
                                if let Ok(op) = reader.read_operator() {
                                    let mem_offset = match op {
                                        wasmparser::Operator::I32Const { value } => value as u32,
                                        wasmparser::Operator::I64Const { value } => value as u32,
                                        _ => continue,
                                    };
                                    data_segments.push(WasmDataSegment {
                                        memory_offset: mem_offset,
                                        data: data.data.to_vec(),
                                    });
                                }
                            }
                        }
                    }
                }
                Payload::CustomSection(custom) => {
                    // Parse "name" section for function names
                    if custom.name() == "name" {
                        let reader =
                            wasmparser::BinaryReader::new(custom.data(), custom.data_offset());
                        let name_reader = NameSectionReader::new(reader);
                        for name in name_reader {
                            if let Ok(Name::Function(map)) = name {
                                for naming in map {
                                    if let Ok(naming) = naming {
                                        func_names.push((naming.index, naming.name.to_string()));
                                    }
                                }
                            }
                        }
                    }
                    sections.push(WasmSection::Other {
                        name: custom.name().to_string(),
                        offset: custom.data_offset(),
                        size: custom.data().len(),
                    });
                }
                _ => {}
            }
        }

        for (idx, name) in func_names {
            if let Some(func) = functions.iter_mut().find(|f| f.index == idx) {
                func.name = Some(name);
            }
        }

        for export in &_exports {
            match export.kind {
                ExportKind::Func => {
                    if let Some(func) = functions.iter_mut().find(|f| f.index == export.index) {
                        if func.name.is_none() {
                            func.name = Some(export.name.clone());
                        }
                    }
                }
                ExportKind::Global => {
                    if let Some(global) = globals.iter_mut().find(|g| g.index == export.index) {
                        if global.name.is_none() {
                            global.name = Some(export.name.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(WasmModule {
            version,
            sections,
            functions,
            globals,
            start_function,
            _exports,
            memory,
            data_segments,
        })
    }
}
