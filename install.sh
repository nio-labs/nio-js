#!/bin/sh
# Install the standalone nio-js executable from an official GitHub release.
set -eu

fail() { printf 'nio-js: %s\n' "$*" >&2; exit 1; }
version=${NIO_JS_VERSION:-latest}
install_dir=${NIO_JS_INSTALL_DIR:-}
while [ "$#" -gt 0 ]; do
  case "$1" in
    --version|--install-dir)
      [ "$#" -ge 2 ] || fail "Missing value for $1"
      case "$1" in
        --version) version=$2 ;;
        --install-dir) install_dir=$2 ;;
      esac
      shift 2 ;;
    -h|--help)
      printf '%s\n' 'Usage: sh install.sh [--version v0.1.0] [--install-dir DIR]' \
        'Defaults: latest stable release; ~/.local/bin ($PREFIX/bin on Termux).' \
        'Environment: NIO_JS_VERSION, NIO_JS_INSTALL_DIR.'
      exit 0 ;;
    *) fail "Unknown option: $1 (use --help)" ;;
  esac
done
[ -n "$version" ] || fail 'Version must not be empty'
if [ "$version" != latest ]; then
  case "$version" in v*) ;; *) version="v$version" ;; esac
  printf '%s\n' "$version" | LC_ALL=C grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9]+([.-][A-Za-z0-9]+)*)?$' || fail 'Expected a version such as v0.1.0 or v0.2.0-rc.1'
fi

os=$(uname -s)
case "$os" in
  Darwin) platform=darwin ;;
  Linux)
    platform=linux
    # Android reports Linux; detect Termux before selecting a glibc build.
    if [ "$(uname -o 2>/dev/null || true)" = Android ] || \
       [ -n "${ANDROID_ROOT:-}" ]; then
      platform=android
    else
      case "$(ldd --version 2>&1 || true)" in
        *musl*) fail 'musl Linux is not supported by the current releases' ;;
      esac
    fi ;;
  *) fail "Unsupported operating system: $os. Windows users can use npm or GitHub Releases." ;;
esac
case "$(uname -m)" in
  x86_64|amd64) arch=x64 ;;
  aarch64|arm64) arch=arm64 ;;
  *) fail "Unsupported architecture: $(uname -m)" ;;
esac
[ "$platform-$arch" != android-x64 ] || fail 'Android releases currently support ARM64 only'
if [ -z "$install_dir" ]; then
  if [ "$platform" = android ] && [ -n "${PREFIX:-}" ]; then
    install_dir="$PREFIX/bin"
  else
    [ -n "${HOME:-}" ] || fail 'Set HOME or use --install-dir'
    install_dir="$HOME/.local/bin"
  fi
fi

download() {
  if command -v curl >/dev/null 2>&1; then
    curl --fail --silent --show-error --location --retry 2 --connect-timeout 15 --max-time 180 "$1" --output "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -q --timeout=30 -O "$2" "$1"
  else
    fail 'Install curl or wget first (Termux: pkg install curl coreutils)'
  fi
}
if command -v sha256sum >/dev/null 2>&1; then
  checksum() { sha256sum "$1"; }
elif command -v shasum >/dev/null 2>&1; then
  checksum() { shasum -a 256 "$1"; }
else
  fail 'Install sha256sum or shasum first (Termux: pkg install coreutils)'
fi

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/nio-js-install.XXXXXX")
staged=
cleanup() {
  rm -rf "$tmp_dir"
  if [ -n "$staged" ]; then rm -f "$staged"; fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
asset="nio-js-$platform-$arch"
base=https://github.com/nio-labs/nio-js/releases
if [ "$version" = latest ]; then
  base="$base/latest/download"
else
  base="$base/download/$version"
fi
printf 'Downloading nio-js (%s, %s)...\n' "$platform-$arch" "$version"
download "$base/$asset" "$tmp_dir/$asset" || fail 'Could not download the binary. Check that this release has been published.'
download "$base/$asset.sha256" "$tmp_dir/checksum" || fail 'Could not download the release checksum'
expected=$(awk 'NR == 1 {print $1}' "$tmp_dir/checksum")
printf '%s\n' "$expected" | LC_ALL=C grep -Eq '^[0-9a-f]{64}$' || fail 'Invalid release checksum'
actual=$(checksum "$tmp_dir/$asset" | awk '{print $1}')
[ "$actual" = "$expected" ] || fail 'Checksum mismatch; installation stopped'
chmod 755 "$tmp_dir/$asset"
"$tmp_dir/$asset" --version || fail 'This binary cannot run on your system; the existing installation was preserved'
mkdir -p "$install_dir" || fail "Cannot create $install_dir; choose a writable --install-dir"
staged=$(mktemp "$install_dir/.nio-js.XXXXXX") || fail "Cannot write to $install_dir"
cp "$tmp_dir/$asset" "$staged"
chmod 755 "$staged"
mv -f "$staged" "$install_dir/nio-js"
staged=
printf 'Installed %s/nio-js\n' "$install_dir"
case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *) printf 'Add this directory to PATH in your shell profile:\n  export PATH="%s:$PATH"\n' "$install_dir" ;;
esac
