# Real-World Example 4: Cloud IoT Worker (Hybrid TS + Go + Raw C)

This example showcases how **The Hybrid Runtime** bridges modern cloud-native networking with legacy hardware parsing, all orchestrated by TypeScript.

## Architecture

- **TypeScript (`server.ts`)**: Handles the HTTP routing and JSON aggregation.
- **Go (`network.go`)**: Brought in via `cgo`, Go provides the ultimate ecosystem for cloud-native networking (gRPC, Kubernetes APIs, Docker SDKs).
- **Raw C (`legacy_parser.c`)**: Compiled into a shared library during preparation using the system C compiler (`cc`, or `CC`). The library is embedded into the capsule.

## Prerequisites

Install Go with cgo enabled and a C compiler (`cc`, or set `CC`). Native libraries are built during preparation and embedded into capsules. Deploy with the same runtime version, OS, and architecture.

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
