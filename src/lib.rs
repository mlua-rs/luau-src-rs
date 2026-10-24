use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents the configuration for building Luau artifacts.
pub struct Build {
    out_dir: Option<PathBuf>,
    target: Option<String>,
    host: Option<String>,
    // Max number of Lua stack slots that a C function can use
    max_cstack_size: usize,
    // Use longjmp instead of C++ exceptions
    use_longjmp: bool,
    // Enable code generator (jit)
    enable_codegen: bool,
    // Enable the compiler
    enable_compiler: bool,
    // Enable the config library
    enable_config: bool,
    // Enable the custom library
    enable_custom: bool,
    // Enable the require library
    enable_require: bool,
    // Enable the VM
    enable_vm: bool,
    // Vector size, must be 3 (default) or 4
    vector_size: usize,
}

/// Represents the artifacts produced by the build process.
pub struct Artifacts {
    lib_dir: PathBuf,
    libs: Vec<String>,
    cpp_stdlib: Option<String>,
    extra_lib_dirs: Vec<PathBuf>,
    extra_libs: Vec<String>,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            out_dir: env::var_os("OUT_DIR").map(PathBuf::from),
            target: env::var("TARGET").ok(),
            host: env::var("HOST").ok(),
            max_cstack_size: 1000000,
            use_longjmp: false,
            enable_codegen: false,
            enable_compiler: true,
            enable_custom: true,
            enable_require: true,
            enable_config: true,
            enable_vm: true,
            vector_size: 3,
        }
    }
}

impl Build {
    /// Creates a new `Build` instance with default settings.
    pub fn new() -> Build {
        Build::default()
    }

    /// Sets the output directory for the build artifacts.
    ///
    /// Default is the environment variable `OUT_DIR`.
    pub fn out_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Build {
        self.out_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets the target architecture for the build.
    ///
    /// Default is the environment variable `TARGET`.
    pub fn target(&mut self, target: &str) -> &mut Build {
        self.target = Some(target.to_string());
        self
    }

    /// Sets the host architecture for the build.
    ///
    /// Default is the environment variable `HOST`.
    pub fn host(&mut self, host: &str) -> &mut Build {
        self.host = Some(host.to_string());
        self
    }

    /// Sets the maximum number of Lua stack slots that a C function can use.
    ///
    /// Default is 1,000,000.
    pub fn set_max_cstack_size(&mut self, size: usize) -> &mut Build {
        self.max_cstack_size = size;
        self
    }

    /// Sets whether to use longjmp instead of C++ exceptions.
    ///
    /// Default is false.
    pub fn use_longjmp(&mut self, r#use: bool) -> &mut Build {
        self.use_longjmp = r#use;
        self
    }

    /// Sets whether to enable the code generator (JIT).
    ///
    /// Default is false.
    pub fn enable_codegen(&mut self, enable: bool) -> &mut Build {
        self.enable_codegen = enable;
        self
    }

    /// Sets whether to enable the compiler (along with the type checker, etc.).
    ///
    /// Default is true.
    pub fn enable_compiler(&mut self, enable: bool) -> &mut Build {
        self.enable_compiler = enable;
        self
    }

    /// Sets whether to enable the config library.
    ///
    /// Default is true.
    pub fn enable_config(&mut self, enable: bool) -> &mut Build {
        self.enable_config = enable;
        self
    }

    /// Sets whether to enable the custom library (for fflag and debug).
    ///
    /// Default is true.
    pub fn enable_custom(&mut self, enable: bool) -> &mut Build {
        self.enable_custom = enable;
        self
    }

    /// Sets whether to enable the require library.
    ///
    /// Default is true.
    pub fn enable_require(&mut self, enable: bool) -> &mut Build {
        self.enable_require = enable;
        self
    }

    /// Sets whether to enable the VM.
    ///
    /// Default is true.
    pub fn enable_vm(&mut self, enable: bool) -> &mut Build {
        self.enable_vm = enable;
        self
    }

    /// Sets the vector size (3 or 4).
    ///
    /// Default is 3.
    pub fn set_vector_size(&mut self, size: usize) -> &mut Build {
        assert!(size == 3 || size == 4, "vector size must be 3 or 4");
        self.vector_size = size;
        self
    }

    /// Builds the Lua artifacts for the specified version.
    pub fn build(&self) -> Artifacts {
        let target = &self.target.as_ref().expect("TARGET is not set")[..];
        let host = &self.host.as_ref().expect("HOST is not set")[..];
        let out_dir = self.out_dir.as_ref().expect("OUT_DIR is not set");
        let build_dir = out_dir.join("luau-build");

        let source_base_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let luau_source_dir = source_base_dir.join("luau");
        let common_include_dir = luau_source_dir.join("Common").join("include");
        let vm_source_dir = luau_source_dir.join("VM").join("src");
        let vm_include_dir = luau_source_dir.join("VM").join("include");

        // Cleanup
        if build_dir.exists() {
            fs::remove_dir_all(&build_dir).unwrap();
        }

        // Configure C++
        let mut config = cc::Build::new();
        config
            .warnings(false)
            .cargo_metadata(false)
            .std("c++17")
            .cpp(true);

        // Extra compiler flags and include directories for the vendored C++.
        //
        // `LUAU_CXXFLAGS` is split on whitespace and appended to the compiler
        // command; `LUAU_INCLUDE_DIRS` uses the platform path separator. Both
        // are declared with `rerun-if-env-changed`, so changing either forces a
        // rebuild instead of silently reusing an artifact built with the old
        // settings.
        println!("cargo:rerun-if-env-changed=LUAU_CXXFLAGS");
        println!("cargo:rerun-if-env-changed=LUAU_INCLUDE_DIRS");
        if let Ok(flags) = env::var("LUAU_CXXFLAGS") {
            for flag in flags.split_whitespace() {
                config.flag(flag);
            }
        }
        if let Ok(dirs) = env::var("LUAU_INCLUDE_DIRS") {
            for dir in env::split_paths(&dirs) {
                config.include(dir);
            }
        }

        let is_wasm_unknown = target == "wasm32-unknown-unknown";
        let wasi = if is_wasm_unknown || target.starts_with("wasm32-wasi") {
            println!("cargo:rerun-if-env-changed=WASI_SYSROOT");

            let sysroot = env::var_os("WASI_SYSROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| panic!("WASI_SYSROOT is not set"));
            let triple = if is_wasm_unknown {
                "wasm32-wasip1"
            } else {
                target
            };

            config
                .flag(format!("--target={}", triple))
                .flag(format!("--sysroot={}", sysroot.display()))
                .flag("-isystem")
                .flag(sysroot.join("include").join(triple).join("eh/c++/v1"))
                .flag("-fexceptions")
                .flag("-fwasm-exceptions")
                .flag("-mllvm")
                .flag("-wasm-use-legacy-eh=false")
                .define("_WASI_EMULATED_PROCESS_CLOCKS", None);
            Some((sysroot, triple))
        } else {
            None
        };

        if target.ends_with("emscripten") {
            // cc-rs unconditionally adds `-fno-exceptions` for wasm32.
            // We need to re-enable exceptions and switch to the
            // wasm-native EH ABI that Rust's emscripten target uses.
            config.flag_if_supported("-fexceptions");
            config.flag_if_supported("-fwasm-exceptions");
        }

        // Common defines
        config.define("LUAI_MAXCSTACK", &*self.max_cstack_size.to_string());
        config.define("LUA_VECTOR_SIZE", &*self.vector_size.to_string());
        config.define("LUA_API", "extern \"C\"");

        if self.use_longjmp {
            config.define("LUA_USE_LONGJMP", "1");
        }

        if cfg!(debug_assertions) {
            config.define("LUAU_ENABLE_ASSERT", None);
        } else {
            // this flag allows compiler to lower sqrt() into a single CPU instruction
            config.flag_if_supported("-fno-math-errno");
        }

        config.include(&common_include_dir);

        // Build `Ast` library
        let ast_lib_name = "luauast";
        let ast_source_dir = luau_source_dir.join("Ast").join("src");
        let ast_include_dir = luau_source_dir.join("Ast").join("include");
        if self.enable_compiler {
            config
                .clone()
                .include(&ast_include_dir)
                .add_files_by_ext_sorted(&ast_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(ast_lib_name);
        }

        // Build `CodeGen` library
        let codegen_lib_name = "luaucodegen";
        let codegen_source_dir = luau_source_dir.join("CodeGen").join("src");
        let codegen_include_dir = luau_source_dir.join("CodeGen").join("include");
        if self.enable_codegen {
            if !self.enable_vm {
                panic!("codegen (jit) requires the VM to be enabled");
            }

            if target.starts_with("wasm") {
                panic!("codegen (jit) is not supported on wasm");
            }

            config
                .clone()
                .include(&codegen_include_dir)
                .include(&vm_include_dir)
                .include(&vm_source_dir)
                .define("LUACODEGEN_API", "extern \"C\"")
                .add_files_by_ext_sorted(&codegen_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(codegen_lib_name);
        }

        // Build `Common` library
        let common_lib_name = "luaucommon";
        let common_source_dir = luau_source_dir.join("Common").join("src");
        config
            .clone()
            .add_files_by_ext_sorted(&common_source_dir, "cpp")
            .out_dir(&build_dir)
            .compile(common_lib_name);

        // Build `Bytecode` library
        let bytecode_lib_name = "luaubytecode";
        let bytecode_source_dir = luau_source_dir.join("Bytecode").join("src");
        let bytecode_include_dir = luau_source_dir.join("Bytecode").join("include");
        let enable_bytecode = self.enable_codegen || self.enable_compiler;
        if enable_bytecode {
            config
                .clone()
                .include(&bytecode_include_dir)
                .add_files_by_ext_sorted(&bytecode_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(bytecode_lib_name);
        }

        // Build `Inliner` library
        let inliner_lib_name = "luauinliner";
        let inliner_source_dir = luau_source_dir.join("Inliner").join("src");
        let inliner_include_dir = luau_source_dir.join("Inliner").join("include");
        if self.enable_codegen {
            config
                .clone()
                .include(&inliner_include_dir)
                .include(&bytecode_include_dir)
                .include(&bytecode_source_dir)
                .include(&vm_include_dir)
                .include(&vm_source_dir)
                .define("LUAJITINLINER_API", "extern \"C\"")
                .add_files_by_ext_sorted(&inliner_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(inliner_lib_name);
        }

        // Build `Compiler` library
        let compiler_lib_name = "luaucompiler";
        let compiler_source_dir = luau_source_dir.join("Compiler").join("src");
        let compiler_include_dir = luau_source_dir.join("Compiler").join("include");
        if self.enable_compiler {
            config
                .clone()
                .include(&compiler_include_dir)
                .include(&bytecode_include_dir)
                .include(&ast_include_dir)
                .define("LUACODE_API", "extern \"C\"")
                .add_files_by_ext_sorted(&compiler_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(compiler_lib_name);
        }

        // Build `Config` library
        let config_lib_name = "luauconfig";
        let config_source_dir = luau_source_dir.join("Config").join("src");
        let config_include_dir = luau_source_dir.join("Config").join("include");
        if self.enable_config {
            if !self.enable_vm {
                panic!("config requires the VM to be enabled");
            }

            config
                .clone()
                .include(&config_include_dir)
                .include(&ast_include_dir)
                .include(&compiler_include_dir)
                .include(&vm_include_dir)
                .add_files_by_ext_sorted(&config_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(config_lib_name);
        }

        // Build customization library
        let custom_lib_name = "luaucustom";
        let custom_source_dir = luau_source_dir.join("Custom").join("src");
        if self.enable_custom {
            if !self.enable_vm {
                panic!("custom requires the VM to be enabled");
            }
            config
                .clone()
                .include(&vm_include_dir)
                .include(&vm_source_dir)
                .add_files_by_ext_sorted(&custom_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(custom_lib_name);
        }

        // Build `Require` library
        let require_lib_name = "luaurequire";
        let require_source_dir = luau_source_dir.join("Require").join("src");
        let require_include_dir = luau_source_dir.join("Require").join("include");
        if self.enable_require {
            if !self.enable_vm {
                panic!("require requires the VM to be enabled");
            }
            config
                .clone()
                .include(&require_include_dir)
                .include(&ast_include_dir)
                .include(&config_include_dir)
                .include(&vm_include_dir)
                .add_files_by_ext_sorted(&require_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(require_lib_name);
        }

        // Build VM
        let vm_lib_name = "luauvm";
        if self.enable_vm {
            config
                .clone()
                .include(&vm_include_dir)
                .add_files_by_ext_sorted(&vm_source_dir, "cpp")
                .out_dir(&build_dir)
                .compile(vm_lib_name);
        }

        let libs = [
            (ast_lib_name, self.enable_compiler),
            (bytecode_lib_name, enable_bytecode),
            (codegen_lib_name, self.enable_codegen),
            (common_lib_name, true),
            (compiler_lib_name, self.enable_compiler),
            (config_lib_name, self.enable_config),
            (custom_lib_name, self.enable_custom),
            (inliner_lib_name, self.enable_codegen),
            (require_lib_name, self.enable_require),
            (vm_lib_name, self.enable_vm),
        ]
        .into_iter()
        .filter_map(|(lib, enabled)| if enabled { Some(lib.to_string()) } else { None })
        .collect::<Vec<_>>();

        let cpp_stdlib = match wasi {
            Some { .. } => None,
            None => Self::get_cpp_link_stdlib(target, host),
        };
        let (extra_lib_dirs, extra_libs) = if let Some((sysroot, triple)) = wasi {
            let lib_dir = sysroot.join("lib").join(triple);
            let extra_lib_dirs = vec![lib_dir.join("eh"), lib_dir];
            let extra_libs = [
                ("c", is_wasm_unknown),
                ("c++", true),
                ("c++abi", true),
                ("unwind", true),
                ("wasi-emulated-process-clocks", true),
            ]
            .into_iter()
            .filter_map(|(lib, enabled)| if enabled { Some(lib.to_string()) } else { None })
            .collect::<Vec<_>>();

            (extra_lib_dirs, extra_libs)
        } else {
            (Vec::new(), Vec::new())
        };

        Artifacts {
            lib_dir: build_dir,
            libs,
            cpp_stdlib,
            extra_lib_dirs,
            extra_libs,
        }
    }

    /// Returns the C++ standard library:
    /// 1) Uses `CXXSTDLIB` environment variable if set
    /// 2) The default `c++` for OS X and BSDs
    /// 3) `c++_shared` for Android
    /// 4) `None` for MSVC
    /// 5) `stdc++` for anything else.
    ///
    /// Inspired by the `cc` crate.
    fn get_cpp_link_stdlib(target: &str, host: &str) -> Option<String> {
        // Try to get value from the `CXXSTDLIB` env variable
        let kind = if host == target { "HOST" } else { "TARGET" };
        let res = env::var(format!("CXXSTDLIB_{target}"))
            .or_else(|_| env::var(format!("CXXSTDLIB_{}", target.replace('-', "_"))))
            .or_else(|_| env::var(format!("{kind}_CXXSTDLIB")))
            .or_else(|_| env::var("CXXSTDLIB"))
            .ok();
        if res.is_some() {
            return res;
        }

        if target.contains("msvc") {
            None
        } else if target.contains("apple") | target.contains("freebsd") | target.contains("openbsd")
        {
            Some("c++".to_string())
        } else if target.contains("android") {
            Some("c++_shared".to_string())
        } else {
            Some("stdc++".to_string())
        }
    }
}

impl Artifacts {
    pub fn lib_dir(&self) -> &Path {
        &self.lib_dir
    }

    pub fn libs(&self) -> &[String] {
        &self.libs
    }

    pub fn print_cargo_metadata(&self) {
        println!("cargo:rustc-link-search=native={}", self.lib_dir.display());
        for dir in &self.extra_lib_dirs {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
        for lib in self.libs.iter().chain(&self.extra_libs) {
            println!("cargo:rustc-link-lib=static={lib}");
        }
        if let Some(ref cpp_stdlib) = self.cpp_stdlib {
            println!("cargo:rustc-link-lib={cpp_stdlib}");
        }
        if let Some(version) = self.version() {
            println!("cargo:rustc-env=LUAU_VERSION={version}");
        }
    }

    pub fn version(&self) -> Option<String> {
        let pkg_version = env!("CARGO_PKG_VERSION");
        let (_, luau_version) = pkg_version.split_once("+luau")?;
        Some(format!("0.{luau_version}"))
    }
}

trait AddFilesByExt {
    fn add_files_by_ext_sorted(&mut self, dir: &Path, ext: &str) -> &mut Self;
}

impl AddFilesByExt for cc::Build {
    // It's important to keep the order of the files to get consistent builds between machines
    // if the order is not always the same, the final binary produces a different SHA256 which
    // might cause issues if one needs to verify which binary is being executed
    fn add_files_by_ext_sorted(&mut self, dir: &Path, ext: &str) -> &mut Self {
        let mut sources: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension() == Some(ext.as_ref()))
            .map(|e| e.path())
            .collect();

        // Sort for determinism
        sources.sort();

        for source in sources {
            self.file(source);
        }

        self
    }
}
