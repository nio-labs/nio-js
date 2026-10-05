# Termux / Android ARM64

nio-js has an Android release target intended for native Termux on 64-bit ARM phones. It uses `aarch64-linux-android` with Android NDK r27c and Bionic, rather than GNU libc Linux binaries. The compiler targets Android API 24 (Android 7), and the executable is linked with 16 KiB ELF segment alignment for newer devices. A 32-bit Termux installation is not covered by this initial target.

CI cross-builds ARM64 and x86_64 Android binaries, then runs the x86_64 build in an Android API 35 emulator. The smoke test exercises version reporting, TypeScript execution, constant and callback HTTP routes, JSON POSTs, timers, File responses, capsule creation/verification, and capsule execution after deleting the source. This checks Android/Bionic behavior but does not verify a real ARM64 phone, Termux's npm installation, Android 7 at runtime, or every device/vendor restriction. Until those checks pass on a device, treat Termux support as a preview.

## Install after a release is published

Use a current Termux installation from its [official installation instructions](https://github.com/termux/termux-app#installation). Run `uname -m`; the ARM64 binary requires `aarch64`.

With Termux's Node package:

```sh
pkg update
pkg install nodejs
npx @nio-labs/nio-js --version
npx @nio-labs/nio-js run app.ts
```

The launcher selects `@nio-labs/nio-js-android-arm64` when Node reports `process.platform === 'android'` and `process.arch === 'arm64'`. Keep optional dependencies enabled. The runtime itself executes JavaScript through QuickJS, not Node.

For the standalone executable, Node is unnecessary:

```sh
pkg install curl coreutils
cd "$HOME"
curl -fLO https://github.com/nio-labs/nio-js/releases/download/v0.1.0/nio-js-android-arm64
curl -fLO https://github.com/nio-labs/nio-js/releases/download/v0.1.0/nio-js-android-arm64.sha256
sha256sum -c nio-js-android-arm64.sha256
install -m 755 nio-js-android-arm64 "$PREFIX/bin/nio-js"
nio-js --version
nio-js run app.ts --host 127.0.0.1 --port 3000
```

Substitute the published version you want. The initial version is `0.1.0`; these URLs become available when that tag is released successfully. Keep the executable in Termux's private filesystem, not `/sdcard` or shared storage. No proot or root is required by the runtime.

## Real-device validation still needed

Check both npm installation and the standalone binary on an ARM64 phone. Confirm `--version`, a TypeScript Hello World service at `127.0.0.1:3000`, callbacks, POST JSON, file uploads, capsule packaging/offline execution, and Ctrl+C shutdown. Test pinned HTTPS imports and outbound HTTPS too: Android/Termux certificate discovery and network behavior are not covered by the emulator smoke test. Background service survival depends on Android's app/process lifecycle and battery management; nio-js is not an Android foreground-service manager.

References: [Rust Android target support](https://doc.rust-lang.org/rustc/platform-support/android.html), [Termux execution environment](https://github.com/termux/termux-packages/wiki/Termux-execution-environment).
