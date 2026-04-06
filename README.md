# Nimble

Nimble is an indentation-sensitive scripting language implemented as a register-based bytecode VM in Rust.
It aims for Python-like readability with a small pragmatic standard library, optional type annotations, colorful diagnostics, and fast startup.

## Current Status

Implemented today:

- Register-based VM execution
- Functions, lambdas, classes/struct-like objects, named arguments
- `if` / `elif` / `else`, `while`, `for`, ranges, `for ... step ...`
- Strings, lists, maps, ranges, interpolation
- Errors as values with `?` propagation
- Local modules plus built-in stdlib modules
- `run`, `check`, and `repl` CLI workflows
- FFI stdlib for calling symbols from single `.dll`, `.so`, and `.dylib` files

Still experimental / limited:

- JIT scaffolding exists but is not wired into normal execution
- FFI currently supports native C ABI calls with primitive scalars, pointers, and C strings
- FFI does not yet support callbacks, variadic functions, or by-value struct marshalling

## Quick Start

Build:

```bash
cargo build --release
```

Create `hello.nmb`:

```nimble
fn main():
    out("Hello, Nimble!")

main()
```

Run it:

```bash
cargo run --release -- run hello.nmb
```

Type-check only:

```bash
cargo run --release -- check hello.nmb
```

Start the REPL:

```bash
cargo run --release -- repl
```

## Language Snapshot

```nimble
fn fib(n int) -> int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

for i in 0..10 step 2:
    out("fib({i}) = {fib(i)}")
```

Nimble supports optional type annotations, named arguments, interpolated strings, module loading, and error propagation:

```nimble
load io

fn first_line(path str) -> str | error:
    lines = io.read_lines(path)?
    if len(lines) == 0:
        return error("empty file")
    return lines[0]

out(first_line("data.txt")?)
```

## Standard Library

The current stdlib includes:

- `io`
- `ffi`
- `json`
- `list`
- `map`
- `math`
- `net`
- `os`
- `path`
- `process`
- `regex`
- `string`
- `time`

The FFI module can load a single dynamic library file directly and call exported symbols:

```nimble
load ffi

lib = ffi.open("./native/mylib.dll")?
result = ffi.call(lib, "add", ["i32", "i32"], "i32", [2, 3])?
out(result)
```

For cross-platform loading:

```nimble
load ffi

lib = ffi.open_any([
    "./native/mylib.dll",
    "./native/libmylib.so",
    "./native/libmylib.dylib",
])?
```

## Examples

The repository ships runnable examples under `examples/` for both core language features and each stdlib module.

- Catalog: [examples/README.md](examples/README.md)
- Syntax reference: [docs/syntax.md](docs/syntax.md)
- Getting started: [docs/getting-started.md](docs/getting-started.md)
- Stdlib docs: [docs/stdlib/](docs/stdlib/)
- Architecture notes: [docs/info/architecture.md](docs/info/architecture.md)
- Performance notes: [docs/info/performance.md](docs/info/performance.md)

Run a single example:

```bash
cargo run --release -- run examples/stdlib/json/roundtrip.nmb
```

On Windows, run the full release example sweep with:

```powershell
.\rae.ps1
```

## Verification

The repo includes integration coverage for:

- shipped examples
- language features like named arguments and stepped ranges
- stdlib behavior
- FFI examples

Typical verification commands:

```bash
cargo test
cargo build --release
```

## License

MIT. See [LICENSE](LICENSE).
