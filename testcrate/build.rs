use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = env::var("TARGET").unwrap();
    let artifacts = luau0_src::Build::new()
        .enable_codegen(!target.starts_with("wasm")) // codegen is not supported on wasm
        .build();
    artifacts.print_cargo_metadata();
}
