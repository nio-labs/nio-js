# Real-World Example 2: AI Agent Worker Service (Hybrid TS + Python)

This example shows how `nio-js` executes **in-process Python AI algorithms** directly inside a TypeScript worker without spawning subprocesses or running external microservices.

## Features Demonstrated
- **Zero-Latency Hybrid Bridge**: TypeScript imports `./agent_kernel.py` functions seamlessly.
- **Intent Categorization**: Real-time natural language query routing.
- **RAG Context Scoring**: Evaluating relevance of retrieved documents for LLM context windows.
- **Task Planning API**: Generating structured task specs for autonomous agent runs.

---

## 🚀 Running the Service

```bash
nio-js run examples/ai-agent-service/server.ts --port 4000
```

---

## 🧪 Testing the Endpoints

### 1. Health & Capabilities
```bash
curl http://localhost:4000/agent/health
```

### 2. Classify Query Intent (In-Process Python)
```bash
curl "http://localhost:4000/agent/classify?prompt=I+need+to+fix+this+segfault+in+rust"
```
```json
{
  "prompt": "I need to fix this segfault in rust",
  "intent": "debugging",
  "confidence": 0.94,
  "routed_tool": "code_debugger"
}
```

### 3. RAG Context Relevance Scoring
```bash
curl -X POST http://localhost:4000/agent/score-context \
  -H "Content-Type: application/json" \
  -d '{
    "document": "QuickJS embedded inside Tokio Rust worker handles HTTP with high throughput.",
    "keywords": ["QuickJS", "Tokio", "Rust", "throughput"]
  }'
```
```json
{
  "document_length": 76,
  "keywords_matched": ["QuickJS", "Tokio", "Rust", "throughput"],
  "relevance_score": 1.0,
  "is_usable": true
}
```

### 4. Task Plan Generation
```bash
curl -X POST http://localhost:4000/agent/plan \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Upgrade Nio runtime to v1.0.0",
    "steps": ["Integrate nio-db", "Expose on() bus", "Deploy cross-platform"],
    "priority": "critical"
  }'
```

---

## 📦 Building an Immutable Capsule

```bash
nio-js build examples/ai-agent-service/server.ts -o dist/ai-agent.njs
nio-js run dist/ai-agent.njs --port 8080
```
