# Releases

The release workflow follows nio-db's native-binary and optional npm-package layout. The launcher package is `@nio-labs/nio-js`; its `nio-js` command runs the matching native executable. Node is needed for this npm launcher, while downloaded GitHub binaries run independently of Node.

## Publishing setup

For the first publication, give this repository access to an npm publishing credential named **NPM_TOKEN**, either as a repository secret or an organization secret available to `nio-labs/nio-js`. It must be authorized to publish public packages under the `@nio-labs` scope and comply with the account's 2FA publishing requirements. No npm credentials are committed to this project.

After the packages exist, you can use [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/) instead: configure each of the seven packages for GitHub organization `nio-labs`, repository `nio-js`, workflow filename `release.yml`, and permission to publish. No GitHub environment is specified by this workflow. Remove the token secret when switching to OIDC. The workflow grants `id-token: write` and uses Node 24 with npm 11 (trusted publishing requires npm 11.5.1+). Provenance is requested for every package. GitHub release publishing uses the workflow's built-in `GITHUB_TOKEN`.

For the manual **Publish to npm** action, add a second trusted publisher for `publish.yml` to every package. npm checks the calling workflow filename for reusable workflows, so trusting only `release.yml` covers tag-triggered releases but not this manual entry point. Both paths can alternatively use `NPM_TOKEN`.

Packages:

* `@nio-labs/nio-js`
* `@nio-labs/nio-js-android-arm64`
* `@nio-labs/nio-js-linux-x64`
* `@nio-labs/nio-js-linux-arm64`
* `@nio-labs/nio-js-darwin-x64`
* `@nio-labs/nio-js-darwin-arm64`
* `@nio-labs/nio-js-win32-x64`

## Release a version

Keep the version in `Cargo.toml`, `package.json`, and all six `optionalDependencies` identical. A version bump needs `cargo check` without `--locked` once to update `Cargo.lock`. Then `npm run check:version` verifies the manifests and lockfile. Versions must be plain semver or a prerelease, such as `0.2.0` or `0.2.0-rc.1`; build metadata is not accepted. Stable releases use npm's `latest` tag; prereleases use `next` and are marked as prereleases on GitHub.

Commit and push the version change, then push its matching tag:

```sh
npm run check:version
npm test
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

The initial package version is `0.1.0`. Use the version actually present in the manifests when creating the tag.

CI and releases pin Rust 1.98.1 for reproducible compiler checks. Every tag starts tests, Clippy, formatting checks, release builds, launcher smoke tests, and npm packaging for six targets. Linux builds use Ubuntu 22.04 with GNU libc, macOS uses native Intel and ARM runners, and Windows uses x64. Android ARM64 is cross-compiled using NDK r27c for API 24 with 16 KiB ELF segment alignment. Android emulator smoke tests gate publishing too. These packages do not target musl Linux, 32-bit Android, iSH, or Windows ARM. Support on older operating systems requires separate compatibility testing. See [TERMUX.md](TERMUX.md) for the Android support scope and device checklist.

Once all builds succeed, the publish job verifies binary SHA-256 digests and npm tarball SHA-512 integrity, publishes platform packages first and the launcher last, then creates a GitHub release containing the six executable assets, checksums, and `install.sh`. Packages install without postinstall downloads or build scripts.

## Publish to npm manually

Pushing a `v*` tag automatically runs `release.yml` and publishes to npm and GitHub. Pushing `main` runs CI only.

For an explicit manual publication, open [Actions → Publish to npm](https://github.com/nio-labs/nio-js/actions/workflows/publish.yml), choose **Run workflow**, and enter an existing version tag such as `v0.1.0`. The action calls the same release pipeline with publishing enabled: it builds all six binaries, runs validation and Android smoke tests, publishes platform packages first, publishes `@nio-labs/nio-js` last, then updates the GitHub release. It forwards the repository's `NPM_TOKEN`; you do not enter credentials in workflow inputs.

You can also start it from the GitHub CLI:

```sh
gh workflow run publish.yml --repo nio-labs/nio-js --ref main -f tag=v0.1.0
```

Manual runs require an **existing version tag** whose source includes the release scripts. For a build/test rehearsal, select `release` and leave its `publish` input unchecked.

Re-running only failed jobs reuses successful build artifacts; rebuilding may produce different bytes and is rejected if that package version already exists. An identical published tarball is skipped; different contents, authentication failures, and registry errors fail the workflow. npm versions are immutable, and publishing several packages plus a GitHub release is not an atomic transaction. If a run partly succeeds, retry the original artifacts or bump the version for changed artifacts.

## Local packaging check

No `npm install` is needed in the source checkout; the packaging scripts use Node's standard library.

```sh
cargo build --release --locked
npm test
node bin/nio-js.cjs --version
npm run package:platform
npm pack --ignore-scripts
```

Build files and tarballs stay in ignored directories. The platform packager accepts explicit binary, OS, and architecture arguments for the release runners. The launcher verifies the installed platform package's version, OS, architecture, and binary checksum; `NIO_JS_BIN` can explicitly override the executable path. Missing optional dependencies produce an actionable installation error rather than an unverified download.
