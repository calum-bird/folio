#!/usr/bin/env sh
set -eu

INSTALL_DIR="${FOLIO_INSTALL_DIR:-$HOME/.local/bin}"
BIN="$INSTALL_DIR/folio"

if [ -x "$BIN" ]; then
  "$BIN" uninstall || true
fi

rm -f "$BIN"
echo "Removed $BIN"
