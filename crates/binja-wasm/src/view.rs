use crate::analysis::{ANALYZED_MODULES, WasmModuleAnalysis};
use crate::arch::WasmRegister;
use crate::wasm::{WasmModule, WasmSection};
use binaryninja::Endianness;
use binaryninja::architecture::{CoreArchitecture, Register};
use binaryninja::binary_view::{BinaryView, BinaryViewBase, BinaryViewExt};
use binaryninja::confidence::Conf;
use binaryninja::custom_binary_view::{
    BinaryViewType, BinaryViewTypeBase, CustomBinaryView, CustomBinaryViewType, CustomView,
    CustomViewBuilder,
};
use binaryninja::low_level_il::LowLevelILTempRegister;
use binaryninja::platform::Platform;
use binaryninja::rc::Ref;
use binaryninja::section::{Section, Semantics};
use binaryninja::segment::{Segment, SegmentFlags};
use binaryninja::types::{FunctionParameter, Type};
use binaryninja::variable::{Variable, VariableSourceType};
use std::sync::Arc;
use tracing::{error, info};

pub struct WasmViewArgs {
    pub data: Vec<u8>,
    pub module: Arc<WasmModule>,
}

pub struct WasmViewType {
    view_type: BinaryViewType,
}

impl WasmViewType {
    pub fn new(view_type: BinaryViewType) -> Self {
        Self { view_type }
    }
}

impl AsRef<BinaryViewType> for WasmViewType {
    fn as_ref(&self) -> &BinaryViewType {
        &self.view_type
    }
}

impl BinaryViewTypeBase for WasmViewType {
    fn is_valid_for(&self, data: &BinaryView) -> bool {
        let mut magic = [0u8; 4];
        if data.read(&mut magic, 0) != 4 {
            return false;
        }
        // WASM magic: \0asm
        magic == [0x00, 0x61, 0x73, 0x6d]
    }

    fn is_deprecated(&self) -> bool {
        false
    }
}

impl CustomBinaryViewType for WasmViewType {
    fn create_custom_view<'builder>(
        &self,
        data: &BinaryView,
        builder: CustomViewBuilder<'builder, Self>,
    ) -> Result<CustomView<'builder>, ()> {
        // Read entire file from parent
        let len = data.len();
        let mut buf = vec![0u8; len as usize];
        data.read(&mut buf, 0);

        // Parse WASM module
        let module = match WasmModule::parse(&buf) {
            Ok(m) => m,
            Err(err) => {
                error!("Failed to parse WASM module: {}", err);
                return Err(());
            }
        };

        info!(
            "Parsed WASM module: version={}, {} sections, {} functions",
            module.version,
            module.sections.len(),
            module.functions.len()
        );

        let module = Arc::new(module);
        let args = WasmViewArgs { data: buf, module };
        builder.create::<WasmView>(data, args)
    }
}

pub struct WasmView {
    handle: Ref<BinaryView>,
    data: Vec<u8>,
    module: Arc<WasmModule>,
}

unsafe impl CustomBinaryView for WasmView {
    type Args = WasmViewArgs;

    fn new(handle: &BinaryView, args: &Self::Args) -> Result<Self, ()> {
        // Use data from args, not from handle (handle points to uninitialized self)
        Ok(Self {
            handle: handle.to_owned(),
            data: args.data.clone(),
            module: Arc::clone(&args.module),
        })
    }

    fn init(&mut self, _args: Self::Args) -> Result<(), ()> {
        let Some(arch) = CoreArchitecture::by_name("wasm") else {
            error!("WASM architecture not found");
            return Err(());
        };
        self.handle.set_default_arch(&arch);

        let Some(platform) = Platform::by_name("wasm") else {
            error!("WASM platform not found");
            return Err(());
        };
        self.handle.set_default_platform(&platform);

        self.handle.begin_bulk_add_segments();

        for section in &self.module.sections {
            match section {
                WasmSection::Code { offset, size, .. } => {
                    if *size > 0 {
                        let segment = Segment::builder(*offset as u64..(*offset + *size) as u64)
                            .parent_backing(*offset as u64..(*offset + *size) as u64)
                            .flags(SegmentFlags::new().readable(true).executable(true))
                            .is_auto(true);
                        self.handle.add_segment(segment);

                        let sec = Section::builder(
                            "Code".to_string(),
                            *offset as u64..(*offset + *size) as u64,
                        )
                        .semantics(Semantics::ReadOnlyCode);
                        self.handle.add_section(sec);
                    }
                }
                WasmSection::Data { offset, size } => {
                    if *size > 0 {
                        let segment = Segment::builder(*offset as u64..(*offset + *size) as u64)
                            .parent_backing(*offset as u64..(*offset + *size) as u64)
                            .flags(SegmentFlags::new().readable(true).writable(true))
                            .is_auto(true);
                        self.handle.add_segment(segment);

                        let sec = Section::builder(
                            "Data".to_string(),
                            *offset as u64..(*offset + *size) as u64,
                        )
                        .semantics(Semantics::ReadWriteData);
                        self.handle.add_section(sec);
                    }
                }
                WasmSection::Other { name, offset, size } => {
                    if *size > 0 {
                        let segment = Segment::builder(*offset as u64..(*offset + *size) as u64)
                            .parent_backing(*offset as u64..(*offset + *size) as u64)
                            .flags(SegmentFlags::new().readable(true))
                            .is_auto(true);
                        self.handle.add_segment(segment);

                        let sec = Section::builder(
                            name.clone(),
                            *offset as u64..(*offset + *size) as u64,
                        )
                        .semantics(Semantics::ReadOnlyData);
                        self.handle.add_section(sec);
                    }
                }
            }
        }

        self.handle.end_bulk_add_segments();

        let mut analyzed_modules = ANALYZED_MODULES.write().unwrap();
        analyzed_modules.register_for_view(
            &self.handle,
            WasmModuleAnalysis::new(Arc::clone(&self.module)),
        );
        let analysis = analyzed_modules.get_for_view_mut(&self.handle).unwrap();

        self.define_functions(analysis);

        info!(
            "Initialized WASM view with {} functions",
            self.module.functions.len()
        );

        Ok(())
    }
}

impl WasmView {
    fn define_functions(&mut self, analysis: &mut WasmModuleAnalysis) {
        let i32_type = Type::int(4, true);

        for func in &self.module.functions {
            if func.code_offset > 0 && func.code_size > 0 {
                let addr = func.code_offset as u64;
                let code = &self.data[func.code_offset..func.code_offset + func.code_size];

                analysis.analyze_function(addr, code, func.param_count);

                self.handle.add_entry_point(addr);

                if let Some(name) = &func.name {
                    let symbol = binaryninja::symbol::Symbol::builder(
                        binaryninja::symbol::SymbolType::Function,
                        name,
                        addr,
                    )
                    .create();
                    self.handle.define_auto_symbol(&symbol);
                }

                let funcs_at = self.handle.functions_at(addr);
                if let Some(bn_func) = funcs_at.iter().next() {
                    info!(
                        "Setting type for function at 0x{:x} with {} params and {} returns",
                        addr, func.param_count, func.return_count
                    );
                    let params: Vec<FunctionParameter> = (0..func.param_count)
                        .map(|i| {
                            let reg_id: i64 = if i < WasmRegister::ARGS.len() {
                                u32::from(WasmRegister::ARGS[i].id()) as i64
                            } else {
                                u32::from(LowLevelILTempRegister::new(i as u32).id()) as i64
                            };
                            FunctionParameter {
                                ty: Conf::new(i32_type.clone(), 255),
                                name: format!("arg{}", i),
                                location: Some(Variable::new(
                                    VariableSourceType::RegisterVariableSourceType,
                                    0,
                                    reg_id,
                                )),
                            }
                        })
                        .collect();

                    let return_type = if func.return_count > 0 {
                        &i32_type
                    } else {
                        &Type::void()
                    };
                    info!("Creating function type with {} parameters", params.len());
                    let func_type = Type::function(return_type, params, false);
                    bn_func.set_user_type(&func_type);
                }
            }
        }
    }
}

impl AsRef<BinaryView> for WasmView {
    fn as_ref(&self) -> &BinaryView {
        &self.handle
    }
}

impl BinaryViewBase for WasmView {
    fn read(&self, buf: &mut [u8], offset: u64) -> usize {
        let offset = offset as usize;
        if offset >= self.data.len() {
            return 0;
        }
        let available = self.data.len() - offset;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.data[offset..offset + to_read]);
        to_read
    }

    fn write(&self, _offset: u64, _data: &[u8]) -> usize {
        0
    }

    fn insert(&self, _offset: u64, _data: &[u8]) -> usize {
        0
    }

    fn remove(&self, _offset: u64, _len: usize) -> usize {
        0
    }

    fn start(&self) -> u64 {
        0
    }

    fn len(&self) -> u64 {
        self.data.len() as u64
    }

    fn entry_point(&self) -> u64 {
        // Return start function if present, otherwise first exported function
        if let Some(start_idx) = self.module.start_function {
            if let Some(func) = self.module.functions.get(start_idx as usize) {
                return func.code_offset as u64;
            }
        }
        // Return first function with code
        for func in &self.module.functions {
            if func.code_offset > 0 {
                return func.code_offset as u64;
            }
        }
        0
    }

    fn default_endianness(&self) -> Endianness {
        Endianness::LittleEndian
    }

    fn address_size(&self) -> usize {
        4
    }

    fn offset_valid(&self, offset: u64) -> bool {
        offset < self.data.len() as u64
    }

    fn offset_readable(&self, offset: u64) -> bool {
        offset < self.data.len() as u64
    }

    fn offset_writable(&self, _offset: u64) -> bool {
        false
    }

    fn offset_executable(&self, offset: u64) -> bool {
        // Check if offset is in code section
        for section in &self.module.sections {
            if let WasmSection::Code {
                offset: sec_offset,
                size,
                ..
            } = section
            {
                let start = *sec_offset as u64;
                let end = start + *size as u64;
                if offset >= start && offset < end {
                    return true;
                }
            }
        }
        false
    }

    fn offset_backed_by_file(&self, offset: u64) -> bool {
        offset < self.data.len() as u64
    }

    fn next_valid_offset_after(&self, offset: u64) -> u64 {
        if offset < self.data.len() as u64 {
            offset + 1
        } else {
            offset
        }
    }
}
