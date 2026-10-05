# Build and deploy a service

nio-js is currently a preview. Validate your application and target operating system before relying on it in production. The following deployment uses a Linux server with GNU libc and systemd. Android/Termux has separate lifecycle constraints; see [TERMUX.md](TERMUX.md).

## Install the runtime

After a release is published:

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio-js/main/install.sh -o install.sh
sh install.sh --version v0.1.0
export PATH="$HOME/.local/bin:$PATH"
nio-js --version
```

Pin a tested runtime version in your build environment and on the server. For the server setup below, install the same release in `/usr/local/bin`:

```sh
sudo sh install.sh --version v0.1.0 --install-dir /usr/local/bin
```

The first release must exist before these downloads work. To build the runtime from source instead, use Rust 1.96+ and a C compiler (`cc` on Linux, Xcode command-line tools on macOS). CI currently uses Rust 1.98.1.

```sh
git clone https://github.com/nio-labs/nio-js.git
cd nio-js
cargo build --release --locked
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/nio-js "$HOME/.local/bin/nio-js"
export PATH="$HOME/.local/bin:$PATH"
nio-js --version
```

Build a native executable for each deployment OS/architecture. A macOS executable cannot run on Linux. The `.njs` application capsule contains portable source and assets and can be used across supported platforms with a compatible runtime.

## Build the application

Create `app.ts`:

```typescript
import { get } from 'nio.js'

get('/', 'Hello World')
get('/healthz', 'ok')
```

Build and validate the capsule:

```sh
mkdir -p dist
nio-js build app.ts -o dist/app.njs
nio-js inspect dist/app.njs
nio-js verify dist/app.njs
nio-js run dist/app.njs --port 3000
```

In another terminal, check `curl --fail http://127.0.0.1:3000/healthz`. Use Ctrl+C to stop the service.

For HTTPS imports, the first build fetches dependencies and creates `nio.lock` beside the entrypoint. Commit the lockfile and pin dependency URLs. For subsequent builds:

```sh
nio-js build app.ts --frozen -o dist/app.njs
# With every dependency already present in the build cache:
nio-js build app.ts --offline --frozen -o dist/app.njs
```

`--frozen` requires an existing lockfile. Only use `--update` during an intentional dependency update. Include local assets explicitly with `--asset name=path`. Runtime outbound networking requires both declared requirements (for example `build --require-net api.example.com`) and matching `run --allow-net api.example.com` grants.

Deploy the capsule, rather than source files or the dependency cache. Capsule execution does not fetch dependencies. Keep a trusted digest in your deployment record: `sha256sum dist/app.njs` on Linux or `shasum -a 256 dist/app.njs` on macOS. `nio-js verify` checks capsule integrity and compatibility; it does not authenticate who published it. Capsules include original source and source maps, so keep secrets out of the build.

## Deploy to Linux

Create a dedicated service account and release directories on the server:

```sh
sudo useradd --system --home /var/lib/nio-js --create-home --shell /usr/sbin/nologin nio-js
sudo install -d -m 755 /opt/nio-app/releases
```

From your build machine, copy the capsule:

```sh
scp dist/app.njs deploy@your-server:/tmp/nio-app.njs
```

On the server, choose a unique release name for every deployment:

```sh
release=2026-10-06-001
sudo install -d -m 755 "/opt/nio-app/releases/$release"
sudo install -m 644 /tmp/nio-app.njs "/opt/nio-app/releases/$release/app.njs"
nio-js verify "/opt/nio-app/releases/$release/app.njs"
# Compare its SHA-256 digest with the trusted build digest before activating.
sha256sum "/opt/nio-app/releases/$release/app.njs"
sudo ln -s "releases/$release" /opt/nio-app/current.next
sudo mv -Tf /opt/nio-app/current.next /opt/nio-app/current
```

Create `/etc/systemd/system/nio-app.service`:

```ini
[Unit]
Description=nio-js application
After=network.target

[Service]
Type=simple
User=nio-js
Group=nio-js
WorkingDirectory=/opt/nio-app
ExecStart=/usr/local/bin/nio-js run /opt/nio-app/current/app.njs --host 127.0.0.1 --port 3000 --memory-mb 64 --timeout-ms 1000 --max-body 1048576
Restart=on-failure
RestartSec=2
TimeoutStopSec=15
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
MemoryMax=256M
TasksMax=64

[Install]
WantedBy=multi-user.target
```

Start the service and inspect its logs:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now nio-app
curl --fail http://127.0.0.1:3000/healthz
sudo systemctl status nio-app
sudo journalctl -u nio-app -f
```

The service logs to stdout/stderr, captured by the journal. systemd sends SIGTERM on stop; nio-js drains requests within a bounded window. If you increase `--timeout-ms`, allow at least that timeout plus five seconds in `TimeoutStopSec`. Adjust process memory limits after measuring your workload; `--memory-mb` limits QuickJS allocations rather than the whole process.

Configure port, host, limits, and networking with CLI flags in `ExecStart`; there is currently no application environment-variable API or `.env` loader. Do not use `process.env`, Node built-ins, or PM2-specific application APIs. General filesystem access and database bindings are also deferred.

## Expose HTTPS

Keep nio-js bound to loopback and put a TLS reverse proxy in front of it. For example, after installing Caddy, point your domain at the server, permit ports 80/443, and put this in `/etc/caddy/Caddyfile`:

```caddyfile
app.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

Reload the Caddy service and check `curl --fail https://app.example.com/healthz`. Public certificate issuance requires the domain and network to be configured correctly. See [Caddy's reverse-proxy guide](https://caddyserver.com/docs/quick-starts/reverse-proxy) for installation and HTTPS requirements. nio-js does not currently terminate TLS itself.

## Update and roll back

Install each new capsule in a new release directory, verify its digest, and atomically replace the `current` symlink using the commands above. Then:

```sh
sudo systemctl restart nio-app
curl --fail http://127.0.0.1:3000/healthz
```

If the health check fails, point `current` back to the previous release directory and restart again. Preserve the runtime version used by that release too; a capsule rollback does not automatically roll back the executable. Runtime updates should be tested separately and pinned with the installer's `--version` option.

This single-process restart briefly interrupts service. Zero-downtime deployment requires a separately managed second instance and switching the reverse proxy after it passes health checks. JavaScript callbacks run serially, and a handler timeout or uncaught failure recycles application state; do not treat in-memory state as durable storage.

For systemd restart and stop behavior, see the [official service reference](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html). Production credentials, authentication, storage, monitoring, and backups depend on your application and must be implemented separately.
