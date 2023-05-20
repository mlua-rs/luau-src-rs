fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let artifacts = luau0_src::Build::new().enable_codegen(true).build();
    artifacts.print_cargo_metadata();
}
