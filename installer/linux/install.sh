#!/bin/sh
# Installs James for the current user. No administrator rights are needed.
#
#   ./install.sh               install, or upgrade an earlier copy
#   ./install.sh --uninstall   remove the app; your conversations in ~/.james stay
set -eu

here=$(cd "$(dirname "$0")" && pwd)
bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
launcher="$data/applications/james.desktop"
icon="$data/icons/hicolor/512x512/apps/james.png"
metainfo="$data/metainfo/com.vers3dynamics.james.metainfo.xml"

refresh_menus() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$data/applications" >/dev/null 2>&1 || true
    fi
}

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$bin/james" "$launcher" "$icon" "$metainfo"
    refresh_menus
    echo "James was removed. Your conversations and settings are still in ~/.james;"
    echo "delete that folder (and ~/.local/share/james, if you set up listening)"
    echo "if you want them gone too, or use Settings > Delete all James data first."
    exit 0
fi

mkdir -p "$bin" "$data/applications" "$(dirname "$icon")" "$(dirname "$metainfo")"
install -m 0755 "$here/james" "$bin/james"
install -m 0644 "$here/james.png" "$icon"
install -m 0644 "$here/com.vers3dynamics.james.metainfo.xml" "$metainfo"
# The menu entry points at this exact copy, so it works even when ~/.local/bin
# is not on the search path.
sed "s|^Exec=.*|Exec=\"$bin/james\"|" "$here/james.desktop" > "$launcher"
chmod 0644 "$launcher"
refresh_menus

echo "James is installed. Open it from your applications menu, or run:"
echo "  $bin/james"
echo "Next, see https://github.com/topherchris420/james/blob/main/docs/GETTING_STARTED.md"
