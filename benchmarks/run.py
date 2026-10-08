#!/usr/bin/env python3
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUITE = Path(__file__).with_name("suite.mjs")
SUITE_NATIVE = Path(__file__).with_name("suite_native.mjs")
NIO = ROOT / "target/release/nio-js"

def execute(name, command, cwd):
    completed = subprocess.run(command, cwd=cwd, text=True, capture_output=True, timeout=120)
    if completed.returncode:
        raise RuntimeError(f"{name} failed:\n{completed.stdout}{completed.stderr}")
    for line in (completed.stdout + "\n" + completed.stderr).splitlines():
        try:
            result = json.loads(line)
            if isinstance(result, dict) and "results" in result:
                return result["results"]
        except json.JSONDecodeError:
            continue
    raise RuntimeError(f"{name} did not print benchmark results")

def main():
    runtimes = []
    SUITE_PYTHON = Path(__file__).with_name("suite_python.mjs")
    SUITE_RUST = Path(__file__).with_name("suite_rust.mjs")
    if NIO.is_file():
        runtimes.append(("NioJS", [str(NIO), "exec", str(SUITE)]))
        runtimes.append(("NioJS (@native)", [str(NIO), "exec", str(SUITE_NATIVE)]))
        runtimes.append(("NioJS (Python)", [str(NIO), "exec", str(SUITE_PYTHON)]))
        runtimes.append(("NioJS (Rust)", [str(NIO), "exec", str(SUITE_RUST)]))
    else:
        print("Build nio-js first!")
        sys.exit(1)
        
    for name, cmd in (("Node", "node"), ("Bun", "bun"), ("Deno", "deno")):
        path = shutil.which(cmd, path=os.environ.get("PATH") + ":" + os.path.expanduser(f"~/.{cmd.lower()}/bin"))
        if path:
            args = [path, "run", "--quiet", str(SUITE)] if name == "Deno" else [path, str(SUITE)]
            runtimes.append((name, args))
            
    print("Running benchmarks...")
    results = {name: execute(name, command, ROOT) for name, command in runtimes}
    
    names = [n for n, _ in runtimes]
    print("\n| Workload | " + " | ".join(f"{name} ops/s" for name in names) + " |")
    print("|---|" + "---:|" * len(names))
    for item in results["NioJS"]:
        rates = []
        for name in names:
            try:
                rates.append(next(x["opsPerSecond"] for x in results[name] if x["name"] == item["name"]))
            except StopIteration:
                rates.append(0)
        print(f"| {item['name']} | " + " | ".join(f"{rate:,}" for rate in rates) + " |")

if __name__ == "__main__":
    main()
