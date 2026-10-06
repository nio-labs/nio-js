# Real-World Example 1: High-Performance REST API Gateway

This example demonstrates building a multi-worker production microservice with `nio-js`.

## Features Demonstrated
- **Sub-millisecond Health Checks**: Immediate JSON serialization without runtime overhead.
- **Dynamic Routing & Path Parameters**: Matching `/api/v1/users/:id`.
- **Query Filtering**: Role and tier filtering on collections.
- **Strict Request Validation**: Error responses with status codes (`400`, `404`, `201`).
- **Hybrid Native Acceleration (`/** @native */`)**: Offloading CPU loops to native Rust.

---

## 🚀 Running the Service

```bash
# 1. Run live development server with 4 worker threads
nio-js run examples/api-gateway/server.ts --port 3000 --workers 4
```

---

## 🧪 Testing the Endpoints

### 1. Health Probe
```bash
curl http://localhost:3000/health
```
```json
{
  "status": "ok",
  "uptime": 12,
  "workers": "pinned-multi-core"
}
```

### 2. List & Filter Users
```bash
# Get all users
curl http://localhost:3000/api/v1/users

# Filter by role
curl "http://localhost:3000/api/v1/users?role=admin"
```

### 3. Fetch Single User
```bash
curl http://localhost:3000/api/v1/users/usr_101
```

### 4. Create New User
```bash
curl -X POST http://localhost:3000/api/v1/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Devin AI", "role": "engineer", "tier": "enterprise"}'
```

### 5. Native Compute Acceleration
```bash
curl "http://localhost:3000/api/v1/metrics/checksum?rounds=100000"
```

---

## 📦 Building a Verified Single-File Capsule

Package into an immutable, offline `.njs` capsule:

```bash
nio-js build examples/api-gateway/server.ts -o dist/api-gateway.njs
nio-js run dist/api-gateway.njs --port 8080
```
