use std::os::raw::{c_char, c_int, c_long, c_void};

#[repr(C)]
#[allow(non_snake_case)]
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
    pub fn luaL_openlibs(state: *mut c_void);
    pub fn lua_getfield(state: *mut c_void, index: c_int, k: *const c_char);
    pub fn lua_tolstring(state: *mut c_void, index: c_int, len: *mut c_long) -> *const c_char;

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
}

pub unsafe fn lua_getglobal(state: *mut c_void, k: *const c_char) {
    lua_getfield(state, -10002 /* LUA_GLOBALSINDEX */, k);
}

#[test]
fn luau_works() {
    use std::{ptr, slice};
    unsafe {
        let state = luaL_newstate();
        assert!(state != ptr::null_mut());

        luaL_openlibs(state);

        let version = {
            lua_getglobal(state, "_VERSION\0".as_ptr().cast());
            let mut len: c_long = 0;
            let version_ptr = lua_tolstring(state, -1, &mut len);
            slice::from_raw_parts(version_ptr as *const u8, len as usize)
        };

        assert_eq!(version, "Luau".as_bytes());

        let code = "function sum(a, b) return a + b end\0";
        let mut bytecode_size = 0;
        let bytecode = luau_compile(
            code.as_ptr().cast(),
            code.len() - 1,
            ptr::null_mut(),
            &mut bytecode_size,
        );
        let result = luau_load(state, "test\0".as_ptr().cast(), bytecode, bytecode_size, 0);
        assert_eq!(result, 0);
        free(bytecode.cast());
    }
}
