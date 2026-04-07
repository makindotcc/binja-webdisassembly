use crate::analysis::{ANALYZED_MODULES, WasmModuleAnalysis};
use crate::arch::WasmRegister;
use crate::wasm::{GlobalValType, WasmModule, WasmSection};
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
use binaryninja::symbol::{Symbol, SymbolType};
use binaryninja::types::{
    FunctionParameter, MemberAccess, MemberScope, Structure, StructureType, Type,
};
use binaryninja::variable::{Variable, VariableSourceType};
use std::sync::Arc;
use tracing::{error, info};

/// Virtual base address for WASM globals segment.
/// Must fit within 32-bit address space (WASM is 32-bit architecture).
pub const GLOBALS_BASE_ADDR: u64 = 0x8000_0000;
/// Size of each global slot (8 bytes to handle i64/f64).
pub const GLOBAL_SLOT_SIZE: u64 = 8;

/// Virtual base address for WASM linear memory.
/// Linear memory addresses from i32.load/i32.store are offset by this base.
pub const LINEAR_MEMORY_BASE: u64 = 0x1000_0000;
/// Size of a WASM memory page (64KB).
pub const WASM_PAGE_SIZE: u64 = 65536;

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

        // Create a virtual segment for globals.
        // This segment has no parent backing, so reads go through our read() method
        // which returns zeros - preventing Binary Ninja from propagating garbage values.
        if !self.module.globals.is_empty() {
            let globals_size = (self.module.globals.len() as u64) * GLOBAL_SLOT_SIZE;
            let globals_end = GLOBALS_BASE_ADDR + globals_size;
            let segment = Segment::builder(GLOBALS_BASE_ADDR..globals_end)
                .flags(SegmentFlags::new().readable(true).writable(true))
                .is_auto(true);
            self.handle.add_segment(segment);

            let sec = Section::builder("Globals".to_string(), GLOBALS_BASE_ADDR..globals_end)
                .semantics(Semantics::ReadWriteData);
            self.handle.add_section(sec);
        }

        // Create a virtual segment for WASM linear memory.
        // This contains data initialized by data segments.
        if let Some(ref memory) = self.module.memory {
            let memory_size = (memory.initial_pages as u64) * WASM_PAGE_SIZE;
            let memory_end = LINEAR_MEMORY_BASE + memory_size;
            let segment = Segment::builder(LINEAR_MEMORY_BASE..memory_end)
                .flags(SegmentFlags::new().readable(true).writable(true))
                .is_auto(true);
            self.handle.add_segment(segment);

            let sec = Section::builder("LinearMemory".to_string(), LINEAR_MEMORY_BASE..memory_end)
                .semantics(Semantics::ReadWriteData);
            self.handle.add_section(sec);

            info!(
                "Created linear memory segment: 0x{:x}-0x{:x} ({} pages, {} data segments)",
                LINEAR_MEMORY_BASE,
                memory_end,
                memory.initial_pages,
                self.module.data_segments.len()
            );
        }

        self.handle.end_bulk_add_segments();

        self.define_types();
        self.define_globals();

        let mut analyzed_modules = ANALYZED_MODULES.write().unwrap();
        analyzed_modules.register_for_view(
            &self.handle,
            WasmModuleAnalysis::new(Arc::clone(&self.module)),
        );
        let analysis = analyzed_modules.get_for_view_mut(&self.handle).unwrap();

        self.define_functions(analysis);

        info!(
            "Initialized WASM view with {} functions and {} globals",
            self.module.functions.len(),
            self.module.globals.len()
        );

        Ok(())
    }
}

impl WasmView {
    /// Define common Go/TinyGo types used in WASM binaries.
    /// These include GoString, GoSlice, and GoInterface.
    fn define_types(&self) {
        // GoString: { ptr *byte, len int }
        let go_string = {
            let mut s = Structure::builder();
            s.structure_type(StructureType::StructStructureType);
            s.insert(
                &Type::pointer_of_width(&Type::int(1, false), 4, false, false, None),
                "ptr",
                0,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
            s.insert(
                &Type::int(4, true),
                "len",
                4,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
            s.width(8);
            Type::structure(&s.finalize())
        };
        self.handle.define_user_type("GoString", &go_string);

        // GoSlice: { ptr *void, len int, cap int }
        let go_slice = {
            let mut s = Structure::builder();
            s.structure_type(StructureType::StructStructureType);
            s.insert(
                &Type::pointer_of_width(&Type::void(), 4, false, false, None),
                "ptr",
                0,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
            s.insert(
                &Type::int(4, true),
                "len",
                4,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
            s.insert(
                &Type::int(4, true),
                "cap",
                8,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
            s.width(12);
            Type::structure(&s.finalize())
        };
        self.handle.define_user_type("GoSlice", &go_slice);

        // GoInterface: { type *void, data *void }
        let go_interface = {
            let mut s = Structure::builder();
            s.structure_type(StructureType::StructStructureType);
            s.insert(
                &Type::pointer_of_width(&Type::void(), 4, false, false, None),
                "type",
                0,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
            s.insert(
                &Type::pointer_of_width(&Type::void(), 4, false, false, None),
                "data",
                4,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
            s.width(8);
            Type::structure(&s.finalize())
        };
        self.handle.define_user_type("GoInterface", &go_interface);

        // GoError: same as GoInterface but named differently
        self.handle.define_user_type("GoError", &go_interface);

        info!("Defined Go/TinyGo types: GoString, GoSlice, GoInterface, GoError");
    }

    fn define_globals(&self) {
        for global in &self.module.globals {
            let addr = GLOBALS_BASE_ADDR + (global.index as u64 * 8);
            let name = global
                .name
                .clone()
                .unwrap_or_else(|| format!("global_{}", global.index));

            let symbol = Symbol::builder(SymbolType::Data, &name, addr).create();
            self.handle.define_auto_symbol(&symbol);

            // Use pointer type for mutable globals (often used as stack pointers)
            let global_type = if global.mutable {
                Type::pointer_of_width(&Type::void(), 4, false, false, None)
            } else {
                match global.val_type {
                    GlobalValType::I32 => Type::int(4, true),
                    GlobalValType::I64 => Type::int(8, true),
                    GlobalValType::F32 => Type::float(4),
                    GlobalValType::F64 => Type::float(8),
                }
            };
            self.handle.define_user_data_var(addr, &global_type);
        }
    }

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

/// Virtual stack base - returned for mutable globals (stack pointers) to prevent
/// Binary Ninja from computing garbage addresses during value analysis.
const VIRTUAL_STACK_BASE: u32 = 0x7FFF_0000;

impl BinaryViewBase for WasmView {
    fn read(&self, buf: &mut [u8], offset: u64) -> usize {
        // Handle reads from globals region
        let globals_end = GLOBALS_BASE_ADDR + (self.module.globals.len() as u64) * GLOBAL_SLOT_SIZE;
        if offset >= GLOBALS_BASE_ADDR && offset < globals_end {
            let available = (globals_end - offset) as usize;
            let to_read = buf.len().min(available);

            // Return a reasonable stack-like value for mutable globals (likely stack pointers)
            let global_idx = ((offset - GLOBALS_BASE_ADDR) / GLOBAL_SLOT_SIZE) as usize;
            if let Some(global) = self.module.globals.get(global_idx) {
                if global.mutable {
                    let stack_val = VIRTUAL_STACK_BASE.to_le_bytes();
                    let copy_len = to_read.min(4);
                    buf[..copy_len].copy_from_slice(&stack_val[..copy_len]);
                    if to_read > 4 {
                        buf[4..to_read].fill(0);
                    }
                    return to_read;
                }
            }
            buf[..to_read].fill(0);
            return to_read;
        }

        // Handle reads from linear memory region
        if let Some(ref memory) = self.module.memory {
            let memory_size = (memory.initial_pages as u64) * WASM_PAGE_SIZE;
            let memory_end = LINEAR_MEMORY_BASE + memory_size;
            if offset >= LINEAR_MEMORY_BASE && offset < memory_end {
                let mem_offset = (offset - LINEAR_MEMORY_BASE) as u32;
                let available = (memory_end - offset) as usize;
                let to_read = buf.len().min(available);

                // Initialize buffer with zeros
                buf[..to_read].fill(0);

                // Fill with data from data segments
                for segment in &self.module.data_segments {
                    let seg_start = segment.memory_offset;
                    let seg_end = seg_start + segment.data.len() as u32;

                    // Check if this segment overlaps with our read range
                    let read_start = mem_offset;
                    let read_end = mem_offset + to_read as u32;

                    if seg_end > read_start && seg_start < read_end {
                        // Calculate overlap
                        let overlap_start = seg_start.max(read_start);
                        let overlap_end = seg_end.min(read_end);

                        let buf_offset = (overlap_start - read_start) as usize;
                        let seg_offset = (overlap_start - seg_start) as usize;
                        let copy_len = (overlap_end - overlap_start) as usize;

                        buf[buf_offset..buf_offset + copy_len]
                            .copy_from_slice(&segment.data[seg_offset..seg_offset + copy_len]);
                    }
                }

                return to_read;
            }
        }

        // Handle reads from file data
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
        if offset < self.data.len() as u64 {
            return true;
        }
        let globals_end = GLOBALS_BASE_ADDR + (self.module.globals.len() as u64) * GLOBAL_SLOT_SIZE;
        if offset >= GLOBALS_BASE_ADDR && offset < globals_end {
            return true;
        }
        if let Some(ref memory) = self.module.memory {
            let memory_end = LINEAR_MEMORY_BASE + (memory.initial_pages as u64) * WASM_PAGE_SIZE;
            if offset >= LINEAR_MEMORY_BASE && offset < memory_end {
                return true;
            }
        }
        false
    }

    fn offset_readable(&self, offset: u64) -> bool {
        if offset < self.data.len() as u64 {
            return true;
        }
        let globals_end = GLOBALS_BASE_ADDR + (self.module.globals.len() as u64) * GLOBAL_SLOT_SIZE;
        if offset >= GLOBALS_BASE_ADDR && offset < globals_end {
            return true;
        }
        if let Some(ref memory) = self.module.memory {
            let memory_end = LINEAR_MEMORY_BASE + (memory.initial_pages as u64) * WASM_PAGE_SIZE;
            if offset >= LINEAR_MEMORY_BASE && offset < memory_end {
                return true;
            }
        }
        false
    }

    fn offset_writable(&self, offset: u64) -> bool {
        let globals_end = GLOBALS_BASE_ADDR + (self.module.globals.len() as u64) * GLOBAL_SLOT_SIZE;
        if offset >= GLOBALS_BASE_ADDR && offset < globals_end {
            return true;
        }
        if let Some(ref memory) = self.module.memory {
            let memory_end = LINEAR_MEMORY_BASE + (memory.initial_pages as u64) * WASM_PAGE_SIZE;
            if offset >= LINEAR_MEMORY_BASE && offset < memory_end {
                return true;
            }
        }
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
