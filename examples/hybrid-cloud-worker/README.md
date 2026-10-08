# Real-World Example 4: Cloud IoT Worker (Hybrid TS + Go + Raw C)

This example showcases how **The Hybrid Runtime** bridges modern cloud-native networking with legacy hardware parsing, all orchestrated by TypeScript.

## Architecture

- **TypeScript (`server.ts`)**: Handles the HTTP routing and JSON aggregation.
- **Go (`network.go`)**: Brought in via `cgo`, Go provides the ultimate ecosystem for cloud-native networking (gRPC, Kubernetes APIs, Docker SDKs).
- **Raw C (`legacy_parser.c`)**: Dynamically compiled **in-memory** by the runtime (using an embedded TinyCC bridge). No toolchains, no Makefiles—just instant execution of legacy C code.

## 🚀 Running the Service

```bash
nio-js run examples/hybrid-cloud-worker/server.ts --port 4002
```

## 🧪 Testing the Endpoint

```bash
curl http://localhost:4002/api/iot/cluster-metrics
```

**Response:**
```json
{
  "source": "hybrid-cloud-worker",
  "timestamp": 1728399581000,
  "infrastructure": {
    "active_nodes": 5,
    "latency_ms": 12,
    "status": "healthy"
  },
  "hardware_telemetry": {
    "raw_voltage": 420.5,
    "normalized_value": 14.2604
  }
}
```

## 📦 Building an Immutable Capsule

```bash
nio-js build examples/hybrid-cloud-worker/server.ts -o dist/cloud-iot.njs
nio-js run dist/cloud-iot.njs --port 8080
```
