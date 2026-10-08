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

pub const FORMAT: u32 = 2;
pub const LOCK_FORMAT: u32 = 1;
pub const MAGIC: &[u8; 4] = b"NJSB";
pub const MAX_MODULE: usize = 16 * 1024 * 1024;
pub const MAX_CAPSULE: usize = 64 * 1024 * 1024;
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Object {
    pub bytecode: Vec<u8>,
    pub integrity: String,
    pub source_integrity: String,
    pub source_map: String,
    pub source_map_integrity: String,
    pub imports: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    pub magic: [u8; 4],
    pub format: u32,
    pub runtime: String,
    pub entry: String,
    pub modules: BTreeMap<String, Object>,
    pub network: Vec<String>,
    pub assets: BTreeMap<String, Asset>,
    pub signature: Option<Vec<u8>>,
    pub publisher: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub body: Vec<u8>,
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

// Comments come from parser spans, so Unicode cannot split an arbitrary byte window.
fn literal_constants(program: &Program) -> BTreeMap<String, serde_json::Value> {
    let mut constants = BTreeMap::new();
    for statement in &program.body {
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => Some(&**declaration),
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => Some(&**declaration),
                _ => None,
            },
            _ => None,
        };
        let Some(declaration) = declaration else {
            continue;
        };
        if declaration.kind != VariableDeclarationKind::Const {
            continue;
        }
        for declarator in &declaration.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            if let Some(Expression::StringLiteral(value)) = &declarator.init
                && !value.lone_surrogates
            {
                constants.insert(id.name.to_string(), serde_json::json!(value.value.as_str()));
            }
        }
    }
    constants
}
fn detect_native_functions(
    program: &Program,
    source: &str,
    ty: SourceType,
) -> Vec<(String, usize, usize, String)> {
    let mut results = Vec::new();
    if source.contains("__nioNative")
        || source.contains("__nioCompileNative")
        || source.contains("__nioInvokeNative")
    {
        return results;
    }
    let constants = literal_constants(program);
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
        let Some(func) = func else { continue };
        let annotated = program.comments.iter().any(|comment| {
            comment.is_leading()
                && comment.span.end as usize <= decl_start
                && source
                    .get(comment.span.end as usize..decl_start)
                    .is_some_and(|gap| gap.trim().is_empty())
                && source
                    .get(comment.span.start as usize..comment.span.end as usize)
                    .is_some_and(|text| {
                        text.split(|c: char| c.is_whitespace() || c == '*')
                            .any(|word| word == "@native")
                    })
        });
        if !annotated {
            continue;
        }
        let Some(body) = &func.body else { continue };
        let Some(function_source) = source.get(func.span.start as usize..func.span.end as usize)
        else {
            continue;
        };
        let Ok((javascript, _, _)) = compile_inner("native.ts", function_source, ty) else {
            continue;
        };
        let (serialized, params, string, json, array_from, builtins, captures, raw_json) =
            if let Ok(plan) = crate::native::numeric(&javascript) {
                (
                    serde_json::to_string(&plan).unwrap(),
                    plan.params,
                    plan.string,
                    false,
                    false,
                    plan.builtins,
                    Vec::new(),
                    false,
                )
            } else if let Ok((plan, captures)) =
                crate::native::json_plan_with_constants(&javascript, &constants)
            {
                let raw_json = plan.raw_compatible();
                (
                    serde_json::to_string(&plan).unwrap(),
                    plan.params,
                    false,
                    true,
                    plan.array_from,
                    plan.builtins,
                    captures,
                    raw_json,
                )
            } else {
                continue;
            };
        // A large native plan must never make an otherwise valid application fail at startup.
        if serialized.len() > 512 * 1024 || results.len() >= 256 {
            continue;
        }
        let name = format!("__nioNative{}", results.len());
        let mut prefix = format!(
            "var {name} = {}({});\n",
            if json {
                "__nioCompileJson"
            } else {
                "__nioCompileNative"
            },
            serde_json::to_string(&serialized).unwrap()
        );
        let args = params.join(",");
        let mut guard = if params.is_empty() {
            "true".into()
        } else {
            params
                .iter()
                .map(|p| format!("typeof {p} === 'number'"))
                .collect::<Vec<_>>()
                .join(" && ")
        };
        guard.push_str(&format!(" && typeof {name} === 'number'"));
        if array_from {
            guard.push_str(
                " && Array === __nioOriginalArray && Array.from === __nioOriginalArrayFrom",
            );
        }
        for builtin in builtins {
            if builtin == "String" {
                guard.push_str(" && String === __nioOriginalString");
            } else {
                guard.push_str(&format!(" && Math === __nioOriginalMath && Math.{builtin} === __nioOriginalMath{builtin}"));
            }
        }
        for capture in captures {
            guard.push_str(&format!(
                " && {capture} === {}",
                serde_json::to_string(&constants[&capture]).unwrap()
            ));
        }
        if json {
            for p in &params {
                guard.push_str(&format!(" && {p} % 1 === 0 && {p} > -1e21 && {p} < 1e21"));
            }
        }
        let invocation = format!(
            "{}({name}, [{args}]){}",
            if json {
                "__nioDecodeNativeJson"
            } else {
                "__nioInvokeNative"
            },
            if string { ".toString()" } else { "" }
        );
        if json
            && raw_json
            && let Some(id) = &func.id
        {
            prefix.push_str(&format!(
                "__nioAttachNativeJson({}, function({args}) {{ if ({guard}) return __nioInvokeJsonRaw({name}, [{args}]); }});\n",
                id.name
            ));
        }
        let start = body.span.start as usize;
        let end = body.span.end as usize;
        let Some(original) = source.get(start + 1..end - 1) else {
            continue;
        };
        let replacement = format!("{{ if ({guard}) return {invocation}; {original} }}");
        results.push((replacement, start, end, prefix));
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
    if let Some(language) = Path::new(&path).extension().and_then(|s| s.to_str())
        && matches!(language, "rs" | "zig" | "c" | "go")
    {
        ensure!(
            !name.starts_with("https://"),
            "native imports must be local trusted sources"
        );
        let wrapper = crate::ffi::compile(source, language)?;
        return compile_inner(name, &wrapper, SourceType::mjs());
    }
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
            let native_fns = detect_native_functions(&parsed.program, source, ty);
            if !native_fns.is_empty() {
                let mut transformed = source.to_string();
                let mut sorted = native_fns;
                sorted.sort_by_key(|a| std::cmp::Reverse(a.1));
                let prefixes = sorted
                    .iter()
                    .map(|item| item.3.as_str())
                    .collect::<String>();
                for (replacement, start, end, _) in sorted {
                    if start < end && end <= transformed.len() {
                        transformed.replace_range(start..end, &replacement);
                    }
                }
                transformed.insert_str(0, &prefixes);
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
struct SourceGraph {
    sources: BTreeMap<String, String>,
    modules: BTreeMap<String, Object>,
}
struct SourceGraphHandle(std::sync::Arc<SourceGraph>);
impl rquickjs::loader::Resolver for SourceGraphHandle {
    fn resolve<'js>(
        &mut self,
        _ctx: &rquickjs::Ctx<'js>,
        base: &str,
        name: &str,
        attributes: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        if attributes.is_some() {
            return Err(rquickjs::Error::new_resolving(base, name));
        }
        if name == "nio.js" {
            return Ok(name.into());
        }
        self.0
            .modules
            .get(base)
            .and_then(|m| m.imports.get(name))
            .cloned()
            .ok_or_else(|| rquickjs::Error::new_resolving(base, name))
    }
}
impl rquickjs::loader::Loader for SourceGraphHandle {
    fn load<'js>(
        &mut self,
        ctx: &rquickjs::Ctx<'js>,
        name: &str,
        attributes: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<rquickjs::Module<'js>> {
        if attributes.is_some() {
            return Err(rquickjs::Error::new_loading(name));
        }
        let source = if name == "nio.js" {
            include_str!("../runtime/nio.js")
        } else {
            self.0
                .sources
                .get(name)
                .ok_or_else(|| rquickjs::Error::new_loading(name))?
                .as_str()
        };
        rquickjs::Module::declare(ctx.clone(), name, source)
    }
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
            format: LOCK_FORMAT,
            transformer: "oxc-0.152".into(),
            remote: BTreeMap::new(),
        }
    };
    ensure!(!options.frozen || existing, "--frozen requires nio.lock");
    ensure!(
        lock.format == LOCK_FORMAT && lock.transformer == "oxc-0.152",
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
    let mut sources = BTreeMap::new();
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

        sources.insert(id.clone(), code);
        let bytecode = Vec::new();

        modules.insert(
            id,
            Object {
                integrity: hash(&bytecode),
                bytecode,
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
    // Compile only after the complete graph and redirect aliases are known.
    let graph = std::sync::Arc::new(SourceGraph {
        sources,
        modules: modules.clone(),
    });
    let rt = rquickjs::Runtime::new()?;
    rt.set_memory_limit(MAX_CAPSULE * 4);
    rt.set_loader(
        SourceGraphHandle(graph.clone()),
        SourceGraphHandle(graph.clone()),
    );
    let ctx = rquickjs::Context::full(&rt)?;
    for (name, object) in &mut modules {
        object.bytecode = ctx.with(|ctx| -> Result<Vec<u8>> {
            use rquickjs::CatchResultExt;
            let module =
                rquickjs::Module::declare(ctx.clone(), name.as_str(), graph.sources[name].as_str())
                    .catch(&ctx)
                    .map_err(|e| anyhow::anyhow!("bytecode compilation {name}: {e}"))?;
            Ok(module.write(rquickjs::module::WriteOptions::default())?)
        })?;
        object.integrity = hash(&object.bytecode);
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
                        body: bytes,
                        media_type: media_type.into()
                    }
                )
                .is_none(),
            "duplicate asset name"
        );
    }
    let capsule = Capsule {
        magic: *MAGIC,
        format: FORMAT,
        runtime: runtime_id(),
        entry: entry_id,
        modules,
        network: options.network.clone(),
        assets,
        signature: None,
        publisher: None,
    };
    validate(&capsule)?;
    Ok(capsule)
}
pub fn validate(c: &Capsule) -> Result<()> {
    ensure!(
        c.magic == *MAGIC && c.format == FORMAT && c.runtime == runtime_id(),
        "incompatible capsule format/runtime"
    );
    ensure!(
        c.modules.len() <= 256 && c.modules.contains_key(&c.entry),
        "invalid capsule entry or module count"
    );
    ensure!(
        bincode::serialize(c)?.len() <= MAX_CAPSULE,
        "capsule too large"
    );
    for (name, object) in &c.modules {
        let url = Url::parse(name)?;
        ensure!(
            matches!(url.scheme(), "https" | "nio-src"),
            "invalid module identifier"
        );
        ensure!(
            object.bytecode.len() <= MAX_MODULE && hash(&object.bytecode) == object.integrity,
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

        for dep in object.imports.values() {
            ensure!(
                dep == "nio.js" || c.modules.contains_key(dep),
                "missing capsule module: {dep}"
            );
        }
    }
    ensure!(c.assets.len() <= 256, "too many assets");
    for (name, asset) in &c.assets {
        ensure!(
            !name.is_empty()
                && !name.starts_with('/')
                && !name.split('/').any(|s| matches!(s, ".." | "." | "")),
            "invalid asset name"
        );
        ensure!(
            asset.body.len() <= MAX_MODULE && hash(&asset.body) == asset.integrity,
            "asset integrity mismatch"
        );
    }
    verify_signature(c, None)?;
    Ok(())
}
pub fn runtime_id() -> String {
    format!(
        "nio-js/{}/qjs-ng-0.14/{}-{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
pub fn capsule_bytes(c: &Capsule) -> Result<Vec<u8>> {
    use bincode::Options;
    Ok(bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(MAX_CAPSULE as u64)
        .serialize(c)?)
}
pub fn decode_capsule(bytes: &[u8]) -> Result<Capsule> {
    use bincode::Options;
    ensure!(
        bytes.starts_with(MAGIC),
        "invalid capsule magic; rebuild legacy JSON capsules"
    );
    let c = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(MAX_CAPSULE as u64)
        .reject_trailing_bytes()
        .deserialize(bytes)?;
    validate(&c)?;
    Ok(c)
}
fn signing_payload(c: &Capsule) -> Result<Vec<u8>> {
    let mut unsigned = c.clone();
    unsigned.signature = None;
    capsule_bytes(&unsigned)
}
pub fn sign(c: &mut Capsule, secret: &[u8]) -> Result<()> {
    use ed25519_dalek::{Signer, SigningKey};
    let bytes: &[u8; 32] = secret
        .try_into()
        .map_err(|_| anyhow::anyhow!("signing key must be 32 bytes"))?;
    let key = SigningKey::from_bytes(bytes);
    c.publisher = Some(key.verifying_key().to_bytes().to_vec());
    c.signature = Some(key.sign(&signing_payload(c)?).to_bytes().to_vec());
    Ok(())
}
pub fn verify_signature(c: &Capsule, trusted: Option<&[u8]>) -> Result<()> {
    match (&c.signature, &c.publisher) {
        (Some(signature), Some(public)) => {
            use ed25519_dalek::{Signature, VerifyingKey};
            if let Some(trusted) = trusted {
                ensure!(
                    public == trusted,
                    "publisher does not match trusted public key"
                );
            }
            let bytes: &[u8; 32] = public
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid publisher key"))?;
            let key = VerifyingKey::from_bytes(bytes)?;
            let signature = Signature::from_slice(signature)?;
            key.verify_strict(&signing_payload(c)?, &signature)?;
        }
        (None, None) => ensure!(
            trusted.is_none(),
            "trusted verification requires a signed capsule"
        ),
        _ => bail!("incomplete capsule signature"),
    }
    Ok(())
}
pub fn key_path(identity: &str) -> Result<PathBuf> {
    ensure!(
        !identity.is_empty()
            && identity.len() <= 128
            && identity
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"@._-".contains(&b))
            && identity != "."
            && identity != "..",
        "invalid key identity"
    );
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .context("home directory unavailable")?;
    Ok(PathBuf::from(home)
        .join(".nio/keys")
        .join(format!("{identity}.key")))
}
pub fn read_capsule(path: &Path) -> Result<Capsule> {
    ensure!(
        fs::metadata(path)?.len() <= MAX_CAPSULE as u64,
        "capsule too large"
    );
    let bytes = fs::read(path)?;
    let c: Capsule = decode_capsule(&bytes)?;
    ensure!(&c.magic == MAGIC, "invalid capsule magic");
    validate(&c)?;
    Ok(c)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_signatures_and_trusted_publishers() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("main.ts"), "export const value = 42;")?;
        let mut c = prepare(&dir.path().join("main.ts"), &Options::default())?;
        let secret = [7; 32];
        sign(&mut c, &secret)?;
        let public = c.publisher.clone().unwrap();
        verify_signature(&c, Some(&public))?;
        assert!(verify_signature(&c, Some(&[8; 32])).is_err());
        let bytes = capsule_bytes(&c)?;
        assert!(bytes.starts_with(MAGIC));
        decode_capsule(&bytes)?;
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode_capsule(&trailing).is_err());
        assert!(decode_capsule(&bytes[..bytes.len() / 2]).is_err());
        c.network.push("example.com".into());
        assert!(validate(&c).is_err());
        Ok(())
    }
    #[test]
    fn key_identities_cannot_escape_the_key_directory() {
        for identity in ["", "../key", "a/b", "a\\b", ".", ".."] {
            assert!(key_path(identity).is_err());
        }
        assert!(key_path("admin@example.com").is_ok());
    }
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
        let bytecode = &c.modules[&c.entry].bytecode;
        assert!(!bytecode.is_empty());
        c.modules.get_mut(&c.entry).unwrap().bytecode.push(0x00);
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
    fn native_directive_matches_code_not_names() -> Result<()> {
        let checksum = "/** @native */\nexport function checksum(iterations: number): number { let acc = 0; for (let i = 0; i < iterations; i++) { acc = (acc * 31 + i) & 0x7fffffff; } return acc; }";
        assert!(
            compile("workload.ts", checksum)?
                .0
                .contains("__nioInvokeNative")
        );
        for source in [
            "const marker = '@native'; function checksum(iterations) { let acc = 0; for (let i = 0; i < iterations; i++) { acc = (acc * 31 + i) & 0x7fffffff; } return acc; }",
            "/** @native */ function other() {} function checksum(iterations) { let acc = 0; for (let i = 0; i < iterations; i++) { acc = (acc * 31 + i) & 0x7fffffff; } return acc; }",
        ] {
            assert!(!compile("workload.js", source)?.0.contains("__nioNative"));
        }
        assert!(
            compile("changed.js", "/** @native */ function cpu() { return 7; }")?
                .0
                .contains("__nioInvokeNative")
        );
        let unicode = format!("// {}x\n{checksum}", "é".repeat(100));
        assert!(
            compile("unicode.ts", &unicode)?
                .0
                .contains("__nioInvokeNative")
        );
        Ok(())
    }
}
