# luau-src
[![Build Status]][github-actions]

[Build Status]: https://github.com/mlua-rs/luau-src-rs/workflows/CI/badge.svg
[github-actions]: https://github.com/mlua-rs/luau-src-rs/actions

This crate contains the sources of [Luau] and logic to build them.
Intended to be consumed by the [mlua-sys] crate.

[Luau]: https://github.com/luau-lang/luau
[mlua-sys]: https://crates.io/crates/mlua-sys

# WebAssembly (WASI)

Applies to the following targets: `wasm32-wasip1`, `wasm32-wasip1-threads`,
`wasm32-wasip2`, and `wasm32-unknown-unknown`.

Requirements are as follows:
* [wasi-sdk] sysroot version 33 or later with C++ exceptions enabled.
* `clang` version 22 or later.
* WASM runtime support for the `exnref` proposal.
* The WASM runtime must support the following `wasi_snapshot_preview1`
  functions:
  * `environ_get`
  * `environ_sizes_get`
  * `clock_time_get`
  * `fd_close` (may be stubbed)
  * `fd_fdstat_get`
  * `fd_seek` (may be stubbed)
  * `fd_write`
  * `proc_exit` (may be stubbed)

Set the `WASI_SYSROOT` environment variable to specify the sysroot path.

When using `wasm32-unknown-unknown`, `__wasm_call_ctors` must be called at
startup, otherwise `wasm-ld` will run the static constructors multiple times,
corrupting the Lua state.

```rust
unsafe extern "C" {
    fn __wasm_call_ctors();
}

#[wasm_bindgen(start)]
fn main() {
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    // SAFETY: This function initializes static C++ constructors and must be
    // called exactly once at startup to prevent repeated initialization. It is
    // called once here.
    unsafe { __wasm_call_ctors() };
}
```

[wasi-sdk]: https://github.com/WebAssembly/wasi-sdk

# License

This project is licensed under [MIT license](http://opensource.org/licenses/MIT)
