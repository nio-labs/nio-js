# Native import contracts

NioJS prepares local native imports by compiling a shared library, embedding its bytes in the module, and generating JavaScript bindings. Rust, Zig, C, and Go all use the C ABI. Native imports are trusted code with process privileges; JavaScript heap, network, and execution limits do not restrict them. Remote native sources are rejected.

Numeric exports accept zero to four double arguments and return a double. Bindings check argument count and numeric conversion. Pointer arguments, structs, integers, variadic functions, asynchronous callbacks, and arbitrary library APIs are unsupported. Keep declarations simple and explicit; macros and complex native signatures are not a binding interface.

## Rust

Install `rustc`. A source file is compiled as an optimized Rust 2021 `cdylib`; Cargo dependencies are not resolved.

```rust
#[unsafe(no_mangle)]
pub extern "C" fn add(a: f64, b: f64) -> f64 { a + b }
```

## Zig

Install `zig` on PATH. The source is compiled with `build-lib -dynamic -O ReleaseFast`.

```zig
export fn multiply(a: f64, b: f64) f64 { return a * b; }
```

## C

Install a C compiler. NioJS runs `cc -shared -fPIC -O3`; `CC` may select another compiler executable. On Windows use a compiler supporting these flags, such as MinGW GCC or Clang. TinyCC is not embedded.

```c
double add(double a, double b) { return a + b; }
```

## Go

Install Go with cgo support and a C compiler. NioJS builds a single source file with `go build -buildmode=c-shared`. Numeric exports use explicit `C.double` arguments and return values. Go modules and multi-file packages are not resolved by this importer.

A zero-argument string export must allocate with `C.CString` and provide exactly this release interface:

```go
package main
/*
#include <stdlib.h>
*/
import "C"
import "unsafe"

//export Text
func Text() *C.char { return C.CString("hello") }

//export FreeCString
func FreeCString(value *C.char) { C.free(unsafe.Pointer(value)) }

func main() {}
```

NioJS copies the returned string into JavaScript and calls `FreeCString` after every successful call. Null results raise an error. Libraries and their temporary files remain owned by the worker and are released when its JavaScript callbacks are dropped.

## Deployment

```typescript
import { add } from './math.c';
import { get } from 'nio.js';
get('/', () => ({ sum: add(1.5, 2.5) }));
```

Preparation needs the native compiler; executing a built capsule does not. Runtime version, OS, CPU architecture, and shared-library dependencies must match the build environment. Python imports separately require the `python` Cargo feature and a compatible Python shared library. Standalone executables package the runtime and capsule but retain those system-library requirements.

Publisher signing is optional. SHA-256 checks detect changed objects but do not establish who produced them. Verify a publisher with `nio-js verify app.njs --public-key publisher.pub` before executing a received capsule, or pass `--public-key publisher.pub` directly to `run` or `exec`. A signature carrying its own public key only proves internal consistency until that key is checked against a trusted key.
