//! WASM module parsing using wasmparser

use eyre::Context;
use wasmparser::{Name, NameSectionReader, Parser, Payload};

#[derive(Debug, Clone)]
pub struct WasmModule {
    pub version: u32,
    pub sections: Vec<WasmSection>,
    pub functions: Vec<WasmFunction>,
    pub start_function: Option<u32>,
    pub _exports: Vec<WasmExport>,
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
}

#[derive(Debug, Clone)]
pub struct WasmExport {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
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
        let mut start_function = None;
        let mut _exports = Vec::new();
        let mut func_index = 0u32;
        let mut import_func_count = 0u32;
        let mut func_names: Vec<(u32, String)> = Vec::new();

        for payload in parser.parse_all(data) {
            let payload = payload.context("parse payload")?;

            match payload {
                Payload::Version { num, .. } => {
                    version = num as u32;
                }
                Payload::ImportSection(reader) => {
                    for import in reader {
                        let import = import.context("parse import")?;
                        if matches!(import.ty, wasmparser::TypeRef::Func(_)) {
                            import_func_count += 1;
                        }
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

                    functions.push(WasmFunction {
                        index: import_func_count + func_index,
                        code_offset: code_start,
                        code_size,
                        name: None,
                    });
                    func_index += 1;
                }
                Payload::DataSection(reader) => {
                    let range = reader.range();
                    sections.push(WasmSection::Data {
                        offset: range.start,
                        size: range.len(),
                    });
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
            if matches!(export.kind, ExportKind::Func)
                && let Some(func) = functions.iter_mut().find(|f| f.index == export.index)
            {
                if func.name.is_none() {
                    func.name = Some(export.name.clone());
                }
            }
        }

        Ok(WasmModule {
            version,
            sections,
            functions,
            start_function,
            _exports,
        })
    }
}
