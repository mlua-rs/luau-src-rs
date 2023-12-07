use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target = env::var("TARGET").unwrap();

    let artifacts = if !target.ends_with("emscripten") {
        luau0_src::Build::new().enable_codegen(true).build()
    } else {
        // llvm generates bytecode for the codegen module which is not supported on wasm
        luau0_src::Build::new().enable_codegen(false).build()
    };
    artifacts.print_cargo_metadata();
}
