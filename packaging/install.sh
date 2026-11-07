#!/bin/sh
# Installs maijp for the current user: binary, menu entry and icon.
set -e
cd "$(dirname "$0")"
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
install -m 755 maijp ~/.local/bin/maijp
install -m 644 dev.maitodesu.maijp.desktop ~/.local/share/applications/
install -m 644 maijp.png ~/.local/share/icons/hicolor/512x512/apps/dev.maitodesu.maijp.png
echo "maijp installed. Launch it from your app menu, or run: maijp"
