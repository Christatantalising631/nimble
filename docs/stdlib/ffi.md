# ffi Module

Foreign-function interface helpers for calling symbols from `.dll`, `.so`, and `.dylib` libraries.

## Functions
- `open(path str) -> ffi_library | error`: Validates and returns a library handle for `path`. Relative paths are resolved against the calling script/module directory.
- `open_any(paths [str]) -> ffi_library | error`: Tries each path in order and returns the first library that loads.
- `close(lib ffi_library | str) -> null | error`: Releases a handle. This is currently a no-op descriptor close.
- `default_c() -> ffi_library | error`: Returns a handle for the platform C runtime (`msvcrt.dll`, `libc.so.6`, or `/usr/lib/libSystem.B.dylib`).
- `default_c_path() -> str | error`: Returns the underlying single-file runtime path/name used by `default_c()`.
- `library_name(base str) -> str | error`: Returns the platform-specific library filename for `base` (`mylib.dll`, `libmylib.so`, or `libmylib.dylib`).
- `call(lib ffi_library | str, symbol str, arg_types [str], ret_type str, args [any]) -> any | error`: Calls `symbol` using the native C ABI.

## Supported Types
- Integers: `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `isize`, `usize`
- Floats: `f32`, `f64`
- Other: `bool`, `ptr`, `str`, `void`

Notes:
- `str` arguments are passed as NUL-terminated C strings.
- `str` return values are read back as `char*` and converted into Nimble strings.
- `ptr` is passed and returned as an integer address.
- Variadic functions are not supported.
- The current implementation uses the native C calling convention on the host platform.

## Example
```nimble
load ffi

lib_path = ffi.default_c_path()?
lib = ffi.open(lib_path)?
length = ffi.call(lib, "strlen", ["str"], "usize", ["nimble"])?
out("loaded {lib_path}")
out("strlen = {length}")
```

## Loading A Single File

You can load a specific dynamic library file directly:

```nimble
load ffi

lib = ffi.open("./native/mylib.dll")?
result = ffi.call(lib, "add", ["i32", "i32"], "i32", [2, 3])?
out(result)
```

On Linux/macOS the same pattern works with `.so` / `.dylib` files. If the path is relative, Nimble resolves it relative to the script that called `ffi.open`.

## Cross-Platform Loading

If your project ships one binary per platform, use `library_name` and/or `open_any`:

```nimble
load ffi
load path

candidate = path.join(["./native", ffi.library_name("mylib")?])
lib = ffi.open(candidate)?
```

Or:

```nimble
load ffi

lib = ffi.open_any([
    "./native/mylib.dll",
    "./native/libmylib.so",
    "./native/libmylib.dylib",
])?
```
