use std::os::raw::{c_char, c_int, c_long, c_void};

#[repr(C)]
#[allow(non_snake_case, non_camel_case_types)]
pub struct lua_CompileOptions {
    optimizationLevel: c_int,
    debugLevel: c_int,
    coverageLevel: c_int,
    vectorLib: *const c_char,
    vectorCtor: *const c_char,
    mutableGlobals: *const *const c_char,
}

extern "C" {
    pub fn free(ptr: *mut c_void);

    pub fn luaL_newstate() -> *mut c_void;
    pub fn lua_close(state: *mut c_void);
    pub fn luaL_openlibs(state: *mut c_void);
    pub fn lua_getfield(state: *mut c_void, index: c_int, k: *const c_char) -> c_int;
    pub fn lua_tolstring(state: *mut c_void, index: c_int, len: *mut c_long) -> *const c_char;
    pub fn lua_call(state: *mut c_void, nargs: c_int, nresults: c_int);

    pub fn lua_pushinteger(state: *mut c_void, n: c_int);
    pub fn lua_tointegerx(state: *mut c_void, index: c_int, isnum: *mut c_int) -> c_int;

    pub fn luau_compile(
        source: *const c_char,
        size: usize,
        options: *mut lua_CompileOptions,
        outsize: *mut usize,
    ) -> *mut c_char;
    pub fn luau_load(
        state: *mut c_void,
        chunkname: *const c_char,
        data: *const c_char,
        size: usize,
        env: c_int,
    ) -> c_int;

    pub fn luau_codegen_supported() -> c_int;
    pub fn luau_codegen_create(state: *mut c_void);
    pub fn luau_codegen_compile(state: *mut c_void, idx: c_int);
}

pub unsafe fn lua_getglobal(state: *mut c_void, k: *const c_char) {
    lua_getfield(state, -102002 /* LUA_GLOBALSINDEX */, k);
}

#[test]
fn luau_works() {
    use std::{ptr, slice};
    unsafe {
        let state = luaL_newstate();
        assert!(state != ptr::null_mut());

        // Enable JIT if supported
        if luau_codegen_supported() != 0 {
            luau_codegen_create(state);
        }

        luaL_openlibs(state);

        let version = {
            lua_getglobal(state, "_VERSION\0".as_ptr().cast());
            let mut len: c_long = 0;
            let version_ptr = lua_tolstring(state, -1, &mut len);
            slice::from_raw_parts(version_ptr as *const u8, len as usize)
        };

        assert_eq!(version, "Luau".as_bytes());

        let code = "local a, b = ... return a + b\0";
        let mut bytecode_size = 0;
        let bytecode = luau_compile(
            code.as_ptr().cast(),
            code.len() - 1,
            ptr::null_mut(),
            &mut bytecode_size,
        );
        let result = luau_load(state, "sum\0".as_ptr().cast(), bytecode, bytecode_size, 0);
        assert_eq!(result, 0);
        free(bytecode.cast());

        // Compile the function (JIT, if supported)
        if luau_codegen_supported() != 0 {
            luau_codegen_compile(state, -1);
        }

        // Call the loaded function
        lua_pushinteger(state, 123);
        lua_pushinteger(state, 321);
        lua_call(state, 2, 1);
        assert_eq!(lua_tointegerx(state, -1, ptr::null_mut()), 444);

        lua_close(state);
    }
}
