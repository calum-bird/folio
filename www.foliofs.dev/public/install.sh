#!/usr/bin/env sh
set -eu

REPO="${FOLIO_REPO:-calum-bird/folio}"
VERSION="${FOLIO_VERSION:-latest}"
INSTALL_DIR="${FOLIO_INSTALL_DIR:-$HOME/.local/bin}"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "folio installer requires $1" >&2
    exit 1
  }
}

target() {
  os="$(uname -s)"
  arch="$(uname -m)"
  if [ "$os" != "Darwin" ]; then
    echo "folio is currently macOS-only" >&2
    exit 1
  fi
  if [ "$arch" != "arm64" ]; then
    echo "folio currently only ships an Apple Silicon build (arm64); detected $arch" >&2
    exit 1
  fi
  echo "aarch64-apple-darwin"
}

download_url() {
  asset="folio-$(target).tar.gz"
  if [ "$VERSION" = "latest" ]; then
    echo "https://github.com/$REPO/releases/latest/download/$asset"
    return
  fi
  echo "https://github.com/$REPO/releases/download/$VERSION/$asset"
}

need curl
need tar
mkdir -p "$INSTALL_DIR"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

url="$(download_url)"
echo "Downloading $url"
curl -fsSL "$url" -o "$tmp/folio.tar.gz"
tar -xzf "$tmp/folio.tar.gz" -C "$tmp"
install -m 0755 "$tmp/folio" "$INSTALL_DIR/folio"

echo "Installed folio to $INSTALL_DIR/folio"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo
    echo "Add this to your shell profile if folio is not on PATH:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac
echo
echo "Run: folio login && folio start"
