#!/usr/bin/env bash
set -euo pipefail
"$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --install 'ndk;27.2.12479018'
toolchain="$ANDROID_HOME/ndk/27.2.12479018/toolchains/llvm/prebuilt/linux-x86_64/bin"
{
  echo "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$toolchain/aarch64-linux-android24-clang"
  echo "CC_aarch64_linux_android=$toolchain/aarch64-linux-android24-clang"
  echo "AR_aarch64_linux_android=$toolchain/llvm-ar"
  echo "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$toolchain/x86_64-linux-android24-clang"
  echo "CC_x86_64_linux_android=$toolchain/x86_64-linux-android24-clang"
  echo "AR_x86_64_linux_android=$toolchain/llvm-ar"
} >> "$GITHUB_ENV"
