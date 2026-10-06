use crate::network::{self, Policy};
use anyhow::{Context, Result, bail, ensure};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

pub const FORMAT: u32 = 1;
pub const MAX_MODULE: usize = 4 * 1024 * 1024;
pub const MAX_CAPSULE: usize = 32 * 1024 * 1024;
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Object {
    pub code: String,
    pub integrity: String,
    pub source_integrity: String,
    pub source_map: String,
    pub source_map_integrity: String,
    pub imports: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    pub format: u32,
    pub runtime: String,
    pub entry: String,
    pub modules: BTreeMap<String, Object>,
    pub network: Vec<String>,
    pub assets: BTreeMap<String, Asset>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub body: String,
    pub integrity: String,
    pub media_type: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    pub format: u32,
    pub transformer: String,
    pub remote: BTreeMap<String, Locked>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Locked {
    pub resolved: String,
    pub integrity: String,
}
pub struct Options {
    pub frozen: bool,
    pub update: bool,
    pub offline: bool,
    pub imports: BTreeMap<String, String>,
    pub policy: Policy,
    pub network: Vec<String>,
    pub cache: PathBuf,
    pub assets: Vec<String>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            frozen: false,
            update: false,
            offline: false,
            imports: BTreeMap::new(),
            policy: Policy {
                hosts: vec!["esm.sh".into(), "unpkg.com".into(), "esm.unpkg.com".into()],
                private: false,
            },
            network: vec![],
            assets: vec![],
            cache: PathBuf::from(".nio-js-cache"),
        }
    }
}
#[derive(Default)]
struct Imports {
    values: Vec<String>,
    computed: bool,
}
impl<'a> Visit<'a> for Imports {
    fn visit_import_declaration(&mut self, d: &ImportDeclaration<'a>) {
        self.values.push(d.source.value.to_string());
    }
    fn visit_export_from_declaration(&mut self, d: &ExportFromDeclaration<'a>) {
        self.values.push(d.source.value.to_string());
    }
    fn visit_export_all_declaration(&mut self, d: &ExportAllDeclaration<'a>) {
        self.values.push(d.source.value.to_string());
    }
    fn visit_import_expression(&mut self, d: &ImportExpression<'a>) {
        if let Expression::StringLiteral(s) = &d.source {
            self.values.push(s.value.to_string());
        } else {
            self.computed = true;
        }
        walk::walk_import_expression(self, d);
    }
}

fn detect_native_functions(program: &Program, source: &str) -> Vec<(String, usize, usize)> {
    if !source.contains("@native") {
        return Vec::new();
    }
    let mut results = Vec::new();
    for stmt in &program.body {
        let (func, decl_start) = match stmt {
            Statement::ExportDeclaration(decl) => {
                if let Declaration::FunctionDeclaration(f) = &decl.declaration {
                    (Some(&**f), decl.span.start as usize)
                } else {
                    (None, 0)
                }
            }
            Statement::FunctionDeclaration(f) => (Some(&**f), f.span.start as usize),
            _ => (None, 0),
        };
        if let Some(oxc_ast::ast::Function {
            id: Some(id),
            body: Some(body),
            span,
            ..
        }) = func
        {
            let func_start = span.start as usize;
            let check_start = decl_start.min(func_start).saturating_sub(150);
            let check_end = func_start.min(source.len());
            if check_start < check_end && source[check_start..check_end].contains("@native") {
                results.push((
                    id.name.to_string(),
                    body.span.start as usize,
                    body.span.end as usize,
                ));
            }
        }
    }
    results
}

fn compile_inner(
    name: &str,
    source: &str,
    ty: SourceType,
) -> Result<(String, Vec<String>, String)> {
    let allocator = Allocator::default();
    let path = Url::parse(name)
        .ok()
        .map(|u| u.path().to_owned())
        .unwrap_or_else(|| name.into());
    let parsed = Parser::new(&allocator, source, ty).parse();
    ensure!(
        parsed.diagnostics.is_empty(),
        "parse error in {name}: {:?}",
        parsed.diagnostics
    );
    let mut program = parsed.program;
    if ty.is_typescript() {
        let semantic = SemanticBuilder::new().build(&program);
        ensure!(
            semantic.diagnostics.is_empty(),
            "semantic error in {name}: {:?}",
            semantic.diagnostics
        );
        let transformed =
            Transformer::new(&allocator, Path::new(&path), &TransformOptions::default())
                .build_with_scoping(semantic.semantic.into_scoping(), &mut program);
        ensure!(
            transformed.diagnostics.is_empty(),
            "TypeScript transform error in {name}: {:?}",
            transformed.diagnostics
        );
    }
    let mut imports = Imports::default();
    imports.visit_program(&program);
    ensure!(
        !imports.computed,
        "computed dynamic import is unsupported in {name}; use literal imports"
    );
    let generated = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some(PathBuf::from(name)),
            ..Default::default()
        })
        .build(&program);
    let map = generated
        .map
        .context("source map missing")?
        .to_json_string();
    Ok((generated.code, imports.values, map))
}

fn compile(name: &str, source: &str) -> Result<(String, Vec<String>, String)> {
    let path = Url::parse(name)
        .ok()
        .map(|u| u.path().to_owned())
        .unwrap_or_else(|| name.into());
    if path.ends_with(".py") {
        let mut js_wrapper = String::from("import { python } from 'nio.js';\n");
        let escaped_code = serde_json::to_string(source)?;
        js_wrapper.push_str(&format!("const __pyCode = {escaped_code};\n"));
        for line in source.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("def ") {
                let fn_name = rest.split('(').next().map(|s| s.trim()).unwrap_or("");
                if !fn_name.is_empty() && fn_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    js_wrapper.push_str(&format!(
                        "export const {fn_name} = python(__pyCode, '{fn_name}');\n"
                    ));
                }
            }
        }
        js_wrapper.push_str("export default new Proxy({}, { get(_, prop) { return python(__pyCode, String(prop)); } });\n");
        return compile_inner(name, &js_wrapper, SourceType::mjs());
    }
    let ty = if path.ends_with(".ts") {
        SourceType::ts()
    } else {
        SourceType::mjs()
    };
    if source.contains("@native") {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, ty).parse();
        if parsed.diagnostics.is_empty() {
            let native_fns = detect_native_functions(&parsed.program, source);
            if !native_fns.is_empty() {
                let mut transformed = source.to_string();
                let mut sorted = native_fns;
                sorted.sort_by_key(|a| std::cmp::Reverse(a.1));
                for (name, start, end) in sorted {
                    if start < end && end <= transformed.len() {
                        transformed.replace_range(
                            start..end,
                            &format!("{{\n  return __nioNative(\"{name}\");\n}}"),
                        );
                    }
                }
                return compile_inner(name, &transformed, ty);
            }
        }
    }
    compile_inner(name, source, ty)
}
fn resolve(base: &str, spec: &str, mappings: &BTreeMap<String, String>) -> Result<String> {
    if spec == "nio.js" {
        return Ok(spec.into());
    }
    let spec = mappings.get(spec).map(String::as_str).unwrap_or(spec);
    if spec.starts_with("https://") {
        return Ok(Url::parse(spec)?.to_string());
    }
    if spec.starts_with("./") || spec.starts_with("../") || spec.starts_with('/') {
        let base_url = Url::parse(base)?;
        if base_url.scheme() == "nio-src" && !spec.starts_with('/') {
            let mut depth = base_url
                .path()
                .split('/')
                .filter(|s| !s.is_empty())
                .count()
                .saturating_sub(1);
            for segment in spec.split('/') {
                let segment = percent_encoding::percent_decode_str(segment).decode_utf8()?;
                if segment == ".." {
                    ensure!(depth > 0, "source import escapes entry directory");
                    depth -= 1;
                } else if segment != "." && !segment.is_empty() {
                    depth += 1;
                }
            }
        }
        let u = base_url.join(spec)?;
        ensure!(
            matches!(u.scheme(), "https" | "nio-src"),
            "unsupported module scheme"
        );
        return Ok(u.to_string());
    }
    bail!("unmapped or unsupported import {spec:?} from {base}")
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".nio-js-{}-{}.tmp",
        std::process::id(),
        hash(bytes)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn prepare(entry: &Path, options: &Options) -> Result<Capsule> {
    ensure!(
        !(options.frozen && options.update),
        "--frozen conflicts with --update"
    );
    ensure!(
        !options.imports.contains_key("nio.js"),
        "nio.js is reserved"
    );
    let entry = entry.canonicalize()?;
    let root = entry.parent().context("entry has no parent")?;
    let mut entry_url = Url::parse("nio-src:///")?;
    entry_url
        .path_segments_mut()
        .map_err(|_| anyhow::anyhow!("invalid source URL"))?
        .push(
            entry
                .file_name()
                .and_then(|s| s.to_str())
                .context("entry filename must be UTF-8")?,
        );
    let entry_id = entry_url.to_string();
    let lock_path = root.join("nio.lock");
    let existing = lock_path.exists();
    if existing {
        ensure!(
            fs::metadata(&lock_path)?.len() <= 1024 * 1024,
            "lockfile too large"
        );
    }
    let mut lock: Lock = if existing {
        serde_json::from_slice(&fs::read(&lock_path)?)?
    } else {
        Lock {
            format: FORMAT,
            transformer: "oxc-0.152".into(),
            remote: BTreeMap::new(),
        }
    };
    ensure!(!options.frozen || existing, "--frozen requires nio.lock");
    ensure!(
        lock.format == FORMAT && lock.transformer == "oxc-0.152",
        "unsupported lock format or transformer"
    );
    for (requested, pinned) in &lock.remote {
        ensure!(
            pinned.integrity.len() == 64 && pinned.integrity.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid lock digest"
        );
        ensure!(
            Url::parse(requested)?.scheme() == "https"
                && Url::parse(&pinned.resolved)?.scheme() == "https",
            "invalid lock URL"
        );
    }
    let original = serde_json::to_vec(&lock)?;
    let mut queue = VecDeque::from([entry_id.clone()]);
    let mut modules = BTreeMap::new();
    let mut aliases = BTreeMap::new();
    let mut total = 0;
    while let Some(requested) = queue.pop_front() {
        if requested == "nio.js"
            || modules.contains_key(&requested)
            || aliases.contains_key(&requested)
        {
            continue;
        }
        ensure!(modules.len() < 256, "module graph exceeds 256 modules");
        let (id, bytes) = if requested.starts_with("nio-src:") {
            let u = Url::parse(&requested)?;
            // URL decoding via file URL keeps encoded path separators subject to canonical root validation.
            let file_url = Url::from_directory_path(root)
                .map_err(|_| anyhow::anyhow!("invalid source root"))?
                .join(u.path().trim_start_matches('/'))?;
            let path = file_url
                .to_file_path()
                .map_err(|_| anyhow::anyhow!("invalid source path"))?
                .canonicalize()?;
            ensure!(
                path.starts_with(root),
                "source import escapes entry directory"
            );
            ensure!(
                fs::metadata(&path)?.len() <= MAX_MODULE as u64,
                "source module too large"
            );
            let bytes = fs::read(path)?;
            (requested.clone(), bytes)
        } else {
            let pinned = if options.update {
                None
            } else {
                lock.remote.get(&requested).cloned()
            };
            if let Some(pinned) = pinned {
                ensure!(
                    options.policy.permits(&Url::parse(&requested)?)
                        && options.policy.permits(&Url::parse(&pinned.resolved)?),
                    "module host denied by preparation policy"
                );
                let path = options.cache.join(&pinned.integrity);
                let bytes = if path.exists() {
                    ensure!(
                        fs::metadata(&path)?.len() <= MAX_MODULE as u64,
                        "cached module too large"
                    );
                    fs::read(path)?
                } else {
                    ensure!(!options.offline, "module missing from cache: {requested}");
                    let d = network::download(
                        &requested,
                        &options.policy,
                        MAX_MODULE,
                        Duration::from_secs(30),
                        true,
                    )?;
                    ensure!(
                        d.status == 200 && d.url == pinned.resolved,
                        "locked module resolution changed: {requested}"
                    );
                    ensure!(
                        hash(&d.bytes) == pinned.integrity,
                        "module integrity mismatch: {requested}"
                    );
                    atomic_write(&path, &d.bytes)?;
                    d.bytes
                };
                ensure!(
                    hash(&bytes) == pinned.integrity,
                    "cached module integrity mismatch: {requested}"
                );
                (pinned.resolved, bytes)
            } else {
                ensure!(
                    !options.offline && !options.frozen && (!existing || options.update),
                    "new dependency requires --update: {requested}"
                );
                let d = network::download(
                    &requested,
                    &options.policy,
                    MAX_MODULE,
                    Duration::from_secs(30),
                    true,
                )?;
                ensure!(
                    d.status == 200,
                    "module download returned HTTP {}: {requested}",
                    d.status
                );
                let digest = hash(&d.bytes);
                let path = options.cache.join(&digest);
                if !path.exists() {
                    atomic_write(&path, &d.bytes)?;
                }
                lock.remote.insert(
                    requested.clone(),
                    Locked {
                        resolved: d.url.clone(),
                        integrity: digest,
                    },
                );
                (d.url, d.bytes)
            }
        };
        ensure!(bytes.len() <= MAX_MODULE, "module too large: {id}");
        total += bytes.len();
        ensure!(total <= MAX_CAPSULE / 2, "module graph too large");
        aliases.insert(requested, id.clone());
        if modules.contains_key(&id) {
            continue;
        }
        let source_integrity = hash(&bytes);
        let source = String::from_utf8(bytes).with_context(|| format!("non-UTF8 module {id}"))?;
        let (code, specs, source_map) = compile(&id, &source)?;
        let mut imports = BTreeMap::new();
        for spec in specs {
            let dep = resolve(&id, &spec, &options.imports)?;
            queue.push_back(dep.clone());
            imports.insert(spec, dep);
        }
        modules.insert(
            id,
            Object {
                integrity: hash(code.as_bytes()),
                code,
                source_integrity,
                source_map_integrity: hash(source_map.as_bytes()),
                source_map,
                imports,
            },
        );
    }
    for object in modules.values_mut() {
        for dep in object.imports.values_mut() {
            if let Some(final_id) = aliases.get(dep) {
                *dep = final_id.clone();
            }
        }
    }
    if options.frozen {
        ensure!(
            serde_json::to_vec(&lock)? == original,
            "frozen lock changed"
        );
    }
    if !options.frozen && (!existing || options.update) {
        atomic_write(&lock_path, &serde_json::to_vec_pretty(&lock)?)?;
    }
    let mut assets = BTreeMap::new();
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    for definition in &options.assets {
        let (name, path) = definition.split_once('=').context("assets use NAME=PATH")?;
        ensure!(
            !name.is_empty()
                && !name.starts_with('/')
                && !name.split('/').any(|s| matches!(s, ".." | "." | "")),
            "invalid asset name"
        );
        ensure!(
            fs::metadata(path)?.len() <= MAX_MODULE as u64,
            "asset too large"
        );
        let bytes = fs::read(path)?;
        let media_type = match Path::new(name).extension().and_then(|s| s.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("svg") => "image/svg+xml",
            Some("css") => "text/css; charset=utf-8",
            Some("json") => "application/json",
            Some("txt") => "text/plain; charset=utf-8",
            Some("png") => "image/png",
            _ => "application/octet-stream",
        };
        ensure!(
            assets
                .insert(
                    name.into(),
                    Asset {
                        integrity: hash(&bytes),
                        body: STANDARD.encode(bytes),
                        media_type: media_type.into()
                    }
                )
                .is_none(),
            "duplicate asset name"
        );
    }
    let capsule = Capsule {
        format: FORMAT,
        runtime: "nio-js/0.1".into(),
        entry: entry_id,
        modules,
        network: options.network.clone(),
        assets,
    };
    validate(&capsule)?;
    Ok(capsule)
}
pub fn validate(c: &Capsule) -> Result<()> {
    ensure!(
        c.format == FORMAT && c.runtime == "nio-js/0.1",
        "incompatible capsule format/runtime"
    );
    ensure!(
        c.modules.len() <= 256 && c.modules.contains_key(&c.entry),
        "invalid capsule entry or module count"
    );
    ensure!(
        serde_json::to_vec(c)?.len() <= MAX_CAPSULE,
        "capsule too large"
    );
    for (name, object) in &c.modules {
        let url = Url::parse(name)?;
        ensure!(
            matches!(url.scheme(), "https" | "nio-src"),
            "invalid module identifier"
        );
        ensure!(
            object.code.len() <= MAX_MODULE && hash(object.code.as_bytes()) == object.integrity,
            "module integrity mismatch: {name}"
        );
        ensure!(
            hash(object.source_map.as_bytes()) == object.source_map_integrity,
            "source map integrity mismatch"
        );
        oxc_sourcemap::SourceMap::from_json_string(&object.source_map)?;
        ensure!(
            object.source_integrity.len() == 64
                && object
                    .source_integrity
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit()),
            "invalid source digest"
        );
        let (_, specs, _) = compile(name.trim_end_matches(".ts"), &object.code)?;
        ensure!(
            specs.iter().all(|s| object.imports.contains_key(s)),
            "capsule has an unrecorded import: {name}"
        );
        for dep in object.imports.values() {
            ensure!(
                dep == "nio.js" || c.modules.contains_key(dep),
                "missing capsule module: {dep}"
            );
        }
    }
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    ensure!(c.assets.len() <= 256, "too many assets");
    for (name, asset) in &c.assets {
        ensure!(
            !name.is_empty()
                && !name.starts_with('/')
                && !name.split('/').any(|s| matches!(s, ".." | "." | "")),
            "invalid asset name"
        );
        let bytes = STANDARD.decode(&asset.body)?;
        ensure!(
            bytes.len() <= MAX_MODULE && hash(&bytes) == asset.integrity,
            "asset integrity mismatch"
        );
    }
    Ok(())
}
pub fn read_capsule(path: &Path) -> Result<Capsule> {
    ensure!(
        fs::metadata(path)?.len() <= MAX_CAPSULE as u64,
        "capsule too large"
    );
    let c = serde_json::from_slice(&fs::read(path)?)?;
    validate(&c)?;
    Ok(c)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ts_graph_and_tamper() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(
            dir.path().join("main.ts"),
            "import { n } from './dep.ts'; import { get } from 'nio.js'; const x: number = n; get('/', () => x)",
        )?;
        fs::write(dir.path().join("dep.ts"), "export const n: number = 42")?;
        let mut c = prepare(&dir.path().join("main.ts"), &Options::default())?;
        assert_eq!(c.modules.len(), 2);
        assert!(!c.modules[&c.entry].code.contains(": number"));
        c.modules.get_mut(&c.entry).unwrap().code.push_str("evil()");
        assert!(validate(&c).is_err());
        Ok(())
    }
    #[test]
    fn encoded_filenames_and_source_boundaries() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("source space #");
        fs::create_dir(&root)?;
        let path = root.join("hello #.ts");
        fs::write(&path, "import {get} from 'nio.js'; get('/','hello')")?;
        let c = prepare(&path, &Options::default())?;
        assert!(c.entry.contains("%23"));
        assert!(resolve("nio-src:///main.ts", "../outside.ts", &BTreeMap::new()).is_err());
        #[cfg(unix)]
        {
            let outside = dir.path().join("outside.ts");
            fs::write(&outside, "export const x=1")?;
            std::os::unix::fs::symlink(outside, root.join("alias.ts"))?;
            fs::write(&path, "import {x} from './alias.ts'; console.log(x)")?;
            assert!(prepare(&path, &Options::default()).is_err());
        }
        Ok(())
    }
    #[test]
    fn unknown_and_computed_import_fail() {
        assert!(compile("x.js", "import(foo)").is_err());
        assert!(resolve("nio-src:///x.js", "node:fs", &BTreeMap::new()).is_err());
        assert!(
            resolve("https://example.com/x.js", "./y.js", &BTreeMap::new())
                .unwrap()
                .ends_with("/y.js")
        );
    }
    #[test]
    fn native_directive_transforms_function() -> Result<()> {
        let code = "/** @native */\nexport function cpu() {\n  let x = 1;\n  for (let i = 0; i < 100; i++) x += i;\n  return x;\n}\n";
        let (compiled, _, _) = compile("workload.js", code)?;
        assert!(compiled.contains("__nioNative(\"cpu\")"));
        Ok(())
    }
}
