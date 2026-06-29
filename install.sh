#!/usr/bin/env sh
# install.sh — build & install vitamin (Linux / macOS only).
#
# Installs the single `vita` binary into ~/.cargo/bin via `cargo install`, then
# creates the `vcmp` / `velab` / `vrun` multicall links next to it (vita
# dispatches on argv[0] basename, so these are just links to the same binary).
#
# Windows is NOT supported (not a build target, not in CI).
set -eu

# Resolve the repo root = directory holding this script, so the script works
# from any cwd.
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

echo "==> Building & installing vita (cargo install --path crates/cli --locked)"
cargo install --path "$SCRIPT_DIR/crates/cli" --locked

# Locate the installed binary. Prefer PATH; fall back to the standard cargo bin
# dir in case ~/.cargo/bin is not yet on PATH for this shell.
VITA=$(command -v vita 2>/dev/null || true)
if [ -z "${VITA}" ]; then
    CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
    if [ -x "$CARGO_BIN/vita" ]; then
        VITA="$CARGO_BIN/vita"
    fi
fi
if [ -z "${VITA}" ] || [ ! -x "${VITA}" ]; then
    echo "error: could not locate the installed 'vita' binary." >&2
    echo "       Ensure ~/.cargo/bin is on your PATH and re-run." >&2
    exit 1
fi

BIN_DIR=$(dirname -- "$VITA")

echo "==> Creating multicall links in $BIN_DIR"
for s in vcmp velab vrun; do
    target="$BIN_DIR/$s"
    # symlink first; fall back to a copy if the filesystem rejects links.
    if ln -sf "$VITA" "$target" 2>/dev/null; then
        echo "    linked  $s -> vita"
    elif cp -f "$VITA" "$target"; then
        echo "    copied  $s (link not supported on this filesystem)"
    else
        echo "error: failed to create '$target'." >&2
        exit 1
    fi
done

echo
echo "Done. Installed: vita, vcmp, velab, vrun  (in $BIN_DIR)"
echo "If 'vita' is not found, add ~/.cargo/bin to your PATH:"
echo '    export PATH="$HOME/.cargo/bin:$PATH"'
