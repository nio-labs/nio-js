//! A deliberately small C ABI: 0–4 double arguments and a double result.
//! Go may also export an owned, zero-argument C string with FreeCString.
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rquickjs::{Ctx, Function, IntoJs, Object, Value};
use std::{
    ffi::{CStr, CString},
    process::Command,
    rc::Rc,
};

#[derive(Debug)]
struct Export {
    name: String,
    arity: usize,
    string: bool,
}
fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit()))
}
fn signature(text: &str, language: &str) -> Result<Export> {
    let (name, rest) = text
        .split_once('(')
        .context("missing native parameter list")?;
    let name = name.trim();
    ensure!(identifier(name), "invalid native export name: {name}");
    let (params, result) = rest
        .split_once(')')
        .context("unterminated native parameters")?;
    let params = params.trim();
    let parameters: Vec<_> = if params.is_empty() || params == "void" {
        vec![]
    } else {
        params.split(',').map(str::trim).collect()
    };
    ensure!(
        parameters.len() <= 4,
        "native export {name} supports at most four arguments"
    );
    for parameter in &parameters {
        let valid = match language {
            "rs" | "zig" => parameter
                .split_once(':')
                .is_some_and(|(n, t)| identifier(n.trim()) && t.trim() == "f64"),
            "c" => parameter
                .strip_prefix("double ")
                .is_some_and(|n| identifier(n.trim())),
            "go" => {
                parameter
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .as_slice()
                    .get(1)
                    == Some(&"C.double")
            }
            _ => false,
        };
        ensure!(
            valid,
            "unsupported parameter in {name}: {parameter}; use f64/double/C.double"
        );
    }
    let result = result.split('{').next().unwrap_or("").trim();
    let string = language == "go" && result == "*C.char";
    ensure!(
        match language {
            "rs" => result == "-> f64",
            "zig" => result == "f64",
            "c" => result.is_empty(),
            "go" => result == "C.double" || string,
            _ => false,
        },
        "unsupported return type in {name}: {result}"
    );
    ensure!(
        !string || parameters.is_empty(),
        "C string exports must have no arguments"
    );
    Ok(Export {
        name: name.into(),
        arity: parameters.len(),
        string,
    })
}
fn exports(source: &str, language: &str) -> Result<Vec<Export>> {
    let mut result = Vec::new();
    match language {
        "rs" | "zig" => {
            let marker = if language == "rs" {
                "pub extern \"C\" fn "
            } else {
                "export fn "
            };
            for text in source.split(marker).skip(1) {
                result.push(signature(text, language)?);
            }
            if language == "rs" {
                ensure!(
                    source.contains("#[no_mangle]") || source.contains("#[unsafe(no_mangle)]"),
                    "Rust exports require #[unsafe(no_mangle)]"
                );
            }
        }
        "c" => {
            for (position, _) in source.match_indices("double ") {
                let text = &source[position + "double ".len()..];
                // Parameter declarations and local variables are not function definitions.
                let Some(open) = text.find('(') else {
                    continue;
                };
                if !identifier(text[..open].trim()) {
                    continue;
                }
                let Some(close) = text.find(')') else {
                    continue;
                };
                if !text[close + 1..].trim_start().starts_with('{') {
                    continue;
                }
                result.push(signature(text, language)?);
            }
        }
        "go" => {
            for text in source.split("//export ").skip(1) {
                let name = text.lines().next().unwrap_or("").trim();
                if name == "FreeCString" {
                    continue;
                }
                let declaration = text
                    .split_once("func ")
                    .context("Go export needs a function declaration")?
                    .1;
                let export = signature(declaration, language)?;
                ensure!(
                    export.name == name,
                    "Go export annotation must match function name"
                );
                result.push(export);
            }
            if result.iter().any(|e| e.string) {
                ensure!(
                    source.contains("//export FreeCString")
                        && source.contains("func FreeCString(value *C.char)"),
                    "owned Go strings require exported func FreeCString(value *C.char)"
                );
            }
        }
        _ => bail!("unsupported native language"),
    }
    ensure!(!result.is_empty(), "no supported native exports found");
    let mut names = std::collections::BTreeSet::new();
    ensure!(
        result.iter().all(|e| names.insert(e.name.clone())),
        "duplicate native export"
    );
    Ok(result)
}
pub fn compile(source: &str, language: &str) -> Result<String> {
    let exports = exports(source, language)?;
    let directory = tempfile::tempdir()?;
    let input = directory.path().join(format!("plugin.{language}"));
    std::fs::write(&input, source)?;
    let output = directory
        .path()
        .join(format!("plugin{}", std::env::consts::DLL_SUFFIX));
    let mut command = match language {
        "rs" => {
            let mut c = Command::new("rustc");
            c.args(["--edition=2021", "--crate-type=cdylib", "-O"])
                .arg(&input)
                .arg("-o")
                .arg(&output);
            c
        }
        "zig" => {
            let mut c = Command::new("zig");
            c.args(["build-lib", "-dynamic", "-O", "ReleaseFast"])
                .arg(format!("-femit-bin={}", output.display()))
                .arg(&input);
            c
        }
        "c" => {
            let mut c = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()));
            c.args(["-shared", "-fPIC", "-O3"])
                .arg(&input)
                .arg("-o")
                .arg(&output);
            c
        }
        "go" => {
            let mut c = Command::new("go");
            c.args(["build", "-buildmode=c-shared", "-o"])
                .arg(&output)
                .arg(&input);
            c
        }
        _ => bail!("unsupported native language"),
    };
    let compilation = command
        .current_dir(directory.path())
        .output()
        .with_context(|| format!("{language} compiler unavailable; install its toolchain"))?;
    ensure!(
        compilation.status.success(),
        "{language} compilation failed:\n{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let bytes = std::fs::read(&output)?;
    ensure!(
        bytes.len() <= crate::prepare::MAX_MODULE,
        "native library exceeds module limit"
    );
    let mut wrapper = format!(
        "const __plugin = globalThis.__loadNative('{}', '{}-{}');\n",
        STANDARD.encode(bytes),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for export in exports {
        wrapper.push_str(&format!("const __{} = __plugin.get('{}', {}, {});\nexport const {} = (...args) => __{}(args);\n", export.name, export.name, export.arity, export.string, export.name, export.name));
    }
    Ok(wrapper)
}
struct Library {
    library: libloading::Library,
    _directory: tempfile::TempDir,
}
fn error(message: impl ToString) -> rquickjs::Error {
    rquickjs::Error::new_loading_message("native library", message.to_string())
}
pub fn loader<'js>(ctx: Ctx<'js>) -> rquickjs::Result<Function<'js>> {
    Function::new(
        ctx,
        move |ctx: Ctx<'js>, encoded: String, target: String| -> rquickjs::Result<Object<'js>> {
            if target != format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH) {
                return Err(error("native library target mismatch"));
            }
            let bytes = STANDARD.decode(encoded).map_err(error)?;
            if bytes.len() > crate::prepare::MAX_MODULE {
                return Err(error("native library too large"));
            }
            let directory = tempfile::tempdir().map_err(error)?;
            let path = directory
                .path()
                .join(format!("plugin{}", std::env::consts::DLL_SUFFIX));
            std::fs::write(&path, bytes).map_err(error)?;
            // Native imports are trusted code, with the privileges of this process.
            let library = unsafe { libloading::Library::new(&path) }.map_err(error)?;
            let library = Rc::new(Library {
                library,
                _directory: directory,
            });
            let object = Object::new(ctx.clone())?;
            object.set("get", Function::new(ctx, move |ctx: Ctx<'js>, name: String, arity: usize, string: bool| -> rquickjs::Result<Function<'js>> {
            if arity > 4 || (string && arity != 0) { return Err(error("unsupported native signature")); }
            let symbol = CString::new(name).map_err(error)?;
            let library = library.clone();
            // Validate the symbol now; each callback retains ownership of its library.
            unsafe { library.library.get::<*const ()>(symbol.as_bytes_with_nul()) }.map_err(error)?;
            if string { unsafe { library.library.get::<unsafe extern "C" fn(*mut std::ffi::c_char)>(b"FreeCString\0") }.map_err(error)?; }
            Function::new(ctx, move |ctx: Ctx<'js>, args: Vec<f64>| -> rquickjs::Result<Value<'js>> {
                if args.len() != arity { return Err(error(format!("expected {arity} numeric arguments"))); }
                let name = symbol.as_bytes_with_nul();
                unsafe {
                    if string {
                        let call = library.library.get::<unsafe extern "C" fn() -> *mut std::ffi::c_char>(name).map_err(error)?;
                        let free = library.library.get::<unsafe extern "C" fn(*mut std::ffi::c_char)>(b"FreeCString\0").map_err(error)?;
                        let pointer = call();
                        if pointer.is_null() { return Err(error("native function returned null")); }
                        let value = CStr::from_ptr(pointer).to_string_lossy().into_owned();
                        free(pointer);
                        return value.into_js(&ctx);
                    }
                    macro_rules! call { ($ty:ty $(, $arg:expr)*) => {{ let f = library.library.get::<$ty>(name).map_err(error)?; f($($arg),*) }} }
                    let value = match arity {
                        0 => call!(unsafe extern "C" fn() -> f64),
                        1 => call!(unsafe extern "C" fn(f64) -> f64, args[0]),
                        2 => call!(unsafe extern "C" fn(f64,f64) -> f64, args[0],args[1]),
                        3 => call!(unsafe extern "C" fn(f64,f64,f64) -> f64, args[0],args[1],args[2]),
                        4 => call!(unsafe extern "C" fn(f64,f64,f64,f64) -> f64, args[0],args[1],args[2],args[3]),
                        _ => unreachable!(),
                    };
                    value.into_js(&ctx)
                }
            })
        })?)?;
            Ok(object)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsupported_abi_before_compilation() {
        assert!(exports("export fn bad(a: i32) f64 { return 0; }", "zig").is_err());
        assert!(exports("double bad(int a) { return 0; }", "c").is_err());
        assert!(exports("//export Text\nfunc Text() *C.char { return nil }", "go").is_err());
        let functions = exports("double add(double a, double b) { return a+b; }", "c").unwrap();
        assert_eq!(functions[0].arity, 2);
        assert_eq!(
            exports("export fn zero() f64 { return 0; }", "zig").unwrap()[0].arity,
            0
        );
    }
}
