#![allow(clippy::missing_safety_doc)]
#![allow(non_snake_case, non_camel_case_types)]

use std::os::raw::{c_char, c_int, c_long, c_void};

#[repr(C)]
pub struct lua_CompileOptions {
    optimizationLevel: c_int,
    debugLevel: c_int,
    typeInfoLevel: c_int,
    coverageLevel: c_int,
    vectorLib: *const c_char,
    vectorCtor: *const c_char,
    vectorType: *const c_char,
    mutableGlobals: *const *const c_char,
    userdataTypes: *const *const c_char,
    librariesWithKnownMembers: *const *const c_char,
    libraryMemberTypeCb: Option<unsafe extern "C" fn(*const c_char, *const c_char) -> c_int>,
    libraryMemberConstantCb:
        Option<unsafe extern "C" fn(*const c_char, *const c_char, *mut *mut c_void)>,
    disabledBuiltins: *const *const c_char,
}

unsafe extern "C" {
    pub fn free(ptr: *mut c_void);

    pub fn luaL_newstate() -> *mut c_void;
    pub fn lua_close(state: *mut c_void);
    pub fn luaL_openlibs(state: *mut c_void);
    pub fn lua_getfield(state: *mut c_void, index: c_int, k: *const c_char) -> c_int;
    pub fn lua_tolstring(state: *mut c_void, index: c_int, len: *mut c_long) -> *const c_char;
    pub fn lua_call(state: *mut c_void, nargs: c_int, nresults: c_int);
    pub fn lua_pcall(state: *mut c_void, nargs: c_int, nresults: c_int, errfunc: c_int) -> c_int;
    pub fn luaL_errorL(state: *mut c_void, format: *const c_char, ...) -> !;

    pub fn lua_pushinteger(state: *mut c_void, n: c_int);
    pub fn lua_tointegerx(state: *mut c_void, index: c_int, isnum: *mut c_int) -> c_int;
    pub fn lua_pushcclosurek(
        L: *mut c_void,
        f: unsafe extern "C-unwind" fn(L: *mut c_void) -> c_int,
        debugname: *const c_char,
        nup: c_int,
        cont: *const c_void,
    );

    pub fn lua_createtable(state: *mut c_void, narr: c_int, nrec: c_int);
    pub fn lua_setmetatable(state: *mut c_void, index: c_int) -> c_int;
    pub fn lua_getmetatable(state: *mut c_void, index: c_int) -> c_int;
    pub fn lua_getmetatablepointer(state: *mut c_void, index: c_int) -> *const c_void;
    pub fn lua_topointer(state: *mut c_void, index: c_int) -> *const c_void;

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

#[cfg(not(target_os = "emscripten"))]
unsafe extern "C" {
    pub fn luau_codegen_supported() -> c_int;
    pub fn luau_codegen_create(state: *mut c_void);
    pub fn luau_codegen_compile(state: *mut c_void, idx: c_int);
}

pub unsafe fn lua_getglobal(state: *mut c_void, k: *const c_char) {
    lua_getfield(state, -1002002 /* LUA_GLOBALSINDEX */, k);
}

pub unsafe fn to_string<'a>(state: *mut c_void, index: c_int) -> &'a str {
    let mut len: c_long = 0;
    let ptr = lua_tolstring(state, index, &mut len);
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    std::str::from_utf8(bytes).unwrap()
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;

    #[test]
    fn test_luau() {
        unsafe {
            let state = luaL_newstate();
            assert!(!state.is_null());

            // Enable JIT if supported
            #[cfg(not(target_os = "emscripten"))]
            if luau_codegen_supported() != 0 {
                luau_codegen_create(state);
            }

            luaL_openlibs(state);

            lua_getglobal(state, c"_VERSION".as_ptr());
            let version = to_string(state, -1);

            assert_eq!(version, "Luau");

            let code = "local a, b = ... return a + b";
            let mut bytecode_size = 0;
            let bytecode = luau_compile(
                code.as_ptr().cast(),
                code.len(),
                ptr::null_mut(),
                &mut bytecode_size,
            );
            let result = luau_load(state, c"sum".as_ptr(), bytecode, bytecode_size, 0);
            assert_eq!(result, 0);
            free(bytecode.cast());

            // Compile the function (JIT, if supported)
            #[cfg(not(target_os = "emscripten"))]
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

    #[test]
    fn test_metatablepointer() {
        unsafe {
            let state = luaL_newstate();
            assert!(!state.is_null());

            lua_createtable(state, 0, 0);
            assert!(lua_getmetatablepointer(state, -1).is_null());

            lua_createtable(state, 0, 0);
            let mt_ptr1 = lua_topointer(state, -1);

            lua_setmetatable(state, -2);
            let mt_ptr2 = lua_getmetatablepointer(state, -1);
            assert_eq!(mt_ptr1, mt_ptr2);

            lua_close(state);
        }
    }

    #[test]
    fn test_exceptions() {
        unsafe {
            let state = luaL_newstate();
            assert!(!state.is_null());

            unsafe extern "C-unwind" fn it_panics(state: *mut c_void) -> c_int {
                luaL_errorL(state, "exception!\0".as_ptr().cast());
            }

            lua_pushcclosurek(state, it_panics, ptr::null(), 0, ptr::null());
            let result = lua_pcall(state, 0, 0, 0);
            assert_eq!(result, 2); // LUA_ERRRUN
            assert_eq!(to_string(state, -1), "exception!");
        }
    }
}
