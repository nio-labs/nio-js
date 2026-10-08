use crate::{engine, network, prepare};
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicUsize};
use std::time::Duration;

pub fn run_mcp_server() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut lines = stdin.lock().lines();

    while let Some(Ok(line)) = lines.next() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "error": { "code": -32700, "message": format!("Parse error: {e}") }
                });
                writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let id = request.get("id").cloned();
        let method = request["method"].as_str().unwrap_or("");

        match method {
            "initialize" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "nio-js",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "notifications/initialized" => {}
            "tools/list" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "nio_check",
                                "description": "Validate a TypeScript/JavaScript file, imports, and capsule syntax with agent-friendly diagnostics",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "file": {
                                            "type": "string",
                                            "description": "Path to the TypeScript or JavaScript file to validate"
                                        }
                                    },
                                    "required": ["file"]
                                }
                            },
                            {
                                "name": "nio_build",
                                "description": "Build a TypeScript or JavaScript service into a portable .njs capsule",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "file": { "type": "string", "description": "Entry file path" },
                                        "output": { "type": "string", "description": "Output .njs capsule path" }
                                    },
                                    "required": ["file", "output"]
                                }
                            },
                            {
                                "name": "nio_eval",
                                "description": "Execute a JavaScript/TypeScript code snippet in a sandboxed nio-js engine and return output",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "code": { "type": "string", "description": "JavaScript or TypeScript code snippet" },
                                        "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default: 1000)" }
                                    },
                                    "required": ["code"]
                                }
                            }
                        ]
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "tools/call" => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("");
                let args = &request["params"]["arguments"];
                let (content, is_error) = handle_tool_call(tool_name, args);
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": content
                            }
                        ],
                        "isError": is_error
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            _ => {
                if id.is_some() {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("Method not found: {method}") }
                    });
                    writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                    stdout.flush()?;
                }
            }
        }
    }
    Ok(())
}

fn handle_tool_call(tool_name: &str, args: &Value) -> (String, bool) {
    match tool_name {
        "nio_check" => {
            let Some(file_str) = args["file"].as_str() else {
                return ("Missing 'file' argument".to_string(), true);
            };
            let path = PathBuf::from(file_str);
            match prepare::prepare(&path, &prepare::Options::default()) {
                Ok(capsule) => {
                    let res = json!({
                        "status": "ok",
                        "file": file_str,
                        "entry": capsule.entry,
                        "modules_count": capsule.modules.len(),
                        "modules": capsule.modules.keys().collect::<Vec<_>>()
                    });
                    (serde_json::to_string_pretty(&res).unwrap(), false)
                }
                Err(e) => {
                    let res = json!({
                        "status": "error",
                        "file": file_str,
                        "message": e.to_string()
                    });
                    (serde_json::to_string_pretty(&res).unwrap(), true)
                }
            }
        }
        "nio_build" => {
            let Some(file_str) = args["file"].as_str() else {
                return ("Missing 'file' argument".to_string(), true);
            };
            let Some(out_str) = args["output"].as_str() else {
                return ("Missing 'output' argument".to_string(), true);
            };
            let input_path = PathBuf::from(file_str);
            let output_path = PathBuf::from(out_str);
            match prepare::prepare(&input_path, &prepare::Options::default()) {
                Ok(capsule) => {
                    match serde_json::to_vec_pretty(&capsule)
                        .map_err(anyhow::Error::from)
                        .and_then(|bytes| prepare::atomic_write(&output_path, &bytes))
                    {
                        Ok(()) => {
                            let res = json!({
                                "status": "ok",
                                "output": out_str,
                                "modules_count": capsule.modules.len()
                            });
                            (serde_json::to_string_pretty(&res).unwrap(), false)
                        }
                        Err(e) => {
                            let res = json!({
                                "status": "error",
                                "message": format!("Failed to write capsule: {e}")
                            });
                            (serde_json::to_string_pretty(&res).unwrap(), true)
                        }
                    }
                }
                Err(e) => {
                    let res = json!({
                        "status": "error",
                        "file": file_str,
                        "message": e.to_string()
                    });
                    (serde_json::to_string_pretty(&res).unwrap(), true)
                }
            }
        }
        "nio_eval" => {
            let Some(code) = args["code"].as_str() else {
                return ("Missing 'code' argument".to_string(), true);
            };
            let timeout_ms = args["timeout_ms"].as_u64().unwrap_or(1000);
            match eval_snippet(code, timeout_ms) {
                Ok(output) => {
                    let res = json!({
                        "status": "ok",
                        "result": output
                    });
                    (serde_json::to_string_pretty(&res).unwrap(), false)
                }
                Err(e) => {
                    let res = json!({
                        "status": "error",
                        "message": e.to_string()
                    });
                    (serde_json::to_string_pretty(&res).unwrap(), true)
                }
            }
        }
        _ => (format!("Unknown tool: {tool_name}"), true),
    }
}

fn eval_snippet(code: &str, timeout_ms: u64) -> Result<String> {
    let temp_dir = tempfile::tempdir()?;
    let script_path = temp_dir.path().join("snippet.js");
    let wrapped = format!(
        "import {{ get, reply }} from 'nio.js';\nget('/', async () => {{\n  const __evalRes = eval({:?});\n  return reply(typeof __evalRes === 'string' ? __evalRes : JSON.stringify(__evalRes));\n}});\n",
        code
    );
    std::fs::write(&script_path, wrapped)?;
    let capsule = prepare::prepare(&script_path, &prepare::Options::default())?;
    let limits = engine::Limits {
        memory: 64 * 1024 * 1024,
        timeout: Duration::from_millis(timeout_ms),
        body: 1024 * 1024,
        policy: network::Policy {
            hosts: vec![],
            private: false,
        },
        in_flight: Arc::new(AtomicUsize::new(0)),
    };
    let mut eng = engine::Engine::new(Arc::new(capsule), limits)?;
    let req = serde_json::json!({
        "method": "GET",
        "url": "/",
        "headers": [],
        "query": {},
        "search": "",
        "params": {},
        "body": "",
        "form": null,
        "requestId": "mcp-eval"
    });
    let reply = eng.dispatch(0, req.to_string())?;
    Ok(String::from_utf8_lossy(&reply.body).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_snippet() -> Result<()> {
        let output = eval_snippet("21 * 2", 1000)?;
        assert_eq!(output, "42");
        Ok(())
    }

    #[test]
    fn test_mcp_tool_check() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let file = directory.path().join("app.js");
        std::fs::write(&file, "export const answer = 42;\n")?;
        let (out, is_err) = handle_tool_call(
            "nio_check",
            &serde_json::json!({"file": file}),
        );
        assert!(!is_err);
        let val: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(val["status"], "ok");
        Ok(())
    }
}
