
mod analysis;
mod arch;
mod decode;
mod view;
mod wasm;

use crate::arch::{WasmArchitecture, WasmCallingConvention};
use crate::view::WasmViewType;
use binaryninja::architecture::{CoreArchitecture, register_architecture};
use binaryninja::calling_convention::register_calling_convention;
use binaryninja::custom_binary_view::register_view_type;
use binaryninja::platform::Platform;
use tracing::info;

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn CorePluginInit() -> bool {
    binaryninja::tracing_init!();

    info!("Registering wasm architecture");
    register_architecture("wasm", WasmArchitecture::new);

    info!("Registering wasm calling convention");
    let arch = CoreArchitecture::by_name("wasm").unwrap();
    let cc = register_calling_convention(&arch, "wasm", WasmCallingConvention);

    info!("Registering wasm platform");
    let platform = Platform::new(&arch, "wasm");
    platform.register_os("wasm");
    platform.set_default_calling_convention(&cc);

    info!("Registering wasm view type");
    register_view_type("WebAssembly", "WebAssembly Module", WasmViewType::new);

    info!("Initialization complete");

    true
}
