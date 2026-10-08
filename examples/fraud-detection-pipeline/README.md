# Real-World Example 3: Fraud Detection Pipeline (Hybrid Zig + Python + Rust + TS)

This example demonstrates the full power of **The Hybrid Runtime** by combining four languages in a single, high-throughput service pipeline without spawning external processes or orchestrating microservices.

## Architecture

- **TypeScript**: Handles HTTP routing, JSON validation, and overall pipeline orchestration.
- **Zig (`crypto.zig`)**: Dynamically compiled via FFI to perform ultra-fast cryptographic signature verification using raw C-ABI.
- **Python (`ml_model.py`)**: Executes an in-process ML inference model to predict transaction fraud probability.
- **Rust (`/** @native */`)**: Offloads computationally heavy map-reduce operations (scanning transaction history) directly to the Rust multi-worker host.

## 🚀 Running the Service

```bash
nio-js run examples/fraud-detection-pipeline/server.ts --port 4001
```

## 🧪 Testing the Endpoint

```bash
curl -X POST http://localhost:4001/api/fraud/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "id": "tx_88192a",
    "amount": 15000.50,
    "user_id_hash": 892341,
    "account_age_days": 14
  }'
```

**Response:**
```json
{
  "transaction_id": "tx_88192a",
  "is_fraud": true,
  "risk_score": 1.45,
  "signals": {
    "signature_valid": false,
    "ml_risk_score": 0.8,
    "history_flags": 3
  }
}
```

## 📦 Building an Immutable Capsule

```bash
nio-js build examples/fraud-detection-pipeline/server.ts -o dist/fraud-analyzer.njs
nio-js run dist/fraud-analyzer.njs --port 8080
```
