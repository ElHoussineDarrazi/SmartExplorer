#!/usr/bin/env bash
#
# package_deb.sh — Paquet .deb Linux pour SmartExplorer (Ubuntu/Debian).
# Installe le binaire, une entrée XDG (.desktop) et l'icône hicolor.
#
# Prérequis : cargo build --release (Linux), dpkg-deb, convert (ImageMagick).
# Variables (toutes optionnelles) :
#   BIN_PATH   binaire release (défaut : target/release/smart_explorer)
#   VERSION    version du paquet (défaut : GITHUB_REF_NAME tag, sinon Cargo.toml)
#   OUT_DIR    dossier de sortie (défaut : dist/)
#   ICON_PNG   icône source (défaut : assets/icons/AppIcon.png)
#
# Sortie : dist/smartexplorer_<version>_<arch>.deb
#
set -euo pipefail

BIN_PATH="${BIN_PATH:-target/release/smart_explorer}"
EXEC_NAME="smartexplorer"
PACKAGE="smartexplorer"
ARCHITECTURE="${ARCHITECTURE:-$(dpkg --print-architecture 2>/dev/null || echo amd64)}"
ICON_PNG="${ICON_PNG:-assets/icons/AppIcon.png}"
OUT_DIR="${OUT_DIR:-dist}"

# Version : fournie > tag Git (vX.Y.Z) > Cargo.toml.
if [[ -z "${VERSION:-}" ]]; then
  REF="${GITHUB_REF_NAME:-}"
  if [[ "$REF" == v* ]]; then
    VERSION="${REF#v}"
  elif grep -m1 -q '^version' Cargo.toml; then
    VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/')"
  fi
fi
VERSION="${VERSION:-0.1.0}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

if [[ ! -f "$BIN_PATH" ]]; then
  echo "ERREUR : binaire introuvable : $BIN_PATH (cargo build --release d'abord)" >&2
  exit 1
fi

echo "============================================================"
echo " Packaging SmartExplorer pour Linux (.deb)"
echo " Binaire : $BIN_PATH | Version : $VERSION | Arch : $ARCHITECTURE"
echo "============================================================"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

mkdir -p "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/icons/hicolor/512x512/apps" \
         "$STAGE/usr/share/doc/$PACKAGE" \
         "$STAGE/DEBIAN"

# 1. Binaire.
install -m 0755 "$BIN_PATH" "$STAGE/usr/bin/$EXEC_NAME"

# 2. Icône 512x512 (standard hicolor).
if command -v convert >/dev/null 2>&1; then
  convert "$ICON_PNG" -resize 512x512 "$STAGE/usr/share/icons/hicolor/512x512/apps/$PACKAGE.png"
else
  echo "AVERTISSEMENT : ImageMagick absent, icône copiée sans redimensionnement."
  cp "$ICON_PNG" "$STAGE/usr/share/icons/hicolor/512x512/apps/$PACKAGE.png"
fi

# 3. Entrée de menu XDG (.desktop).
cat > "$STAGE/usr/share/applications/$PACKAGE.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=SmartExplorer
GenericName=File Manager
Comment=Explorateur de fichiers multiplateforme
Exec=$EXEC_NAME %F
Icon=$PACKAGE
Terminal=false
Categories=Utility;FileManagement;System;
Keywords=files;explorer;manager;ssh;sftp;ftp;
DESKTOP

# 4. Contrôle du paquet (dépendances runtime : X11/GL/xkb pour eframe,
#    libssh2 pour ssh2).
cat > "$STAGE/DEBIAN/control" <<CONTROL
Package: $PACKAGE
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCHITECTURE
Depends: libc6, libgcc-s1, libx11-6, libxcb1, libxkbcommon0, libgl1, libssh2-1t64 | libssh2-1
Maintainer: SmartExplorer <contact@smartexplorer.app>
Homepage: https://github.com/ElHoussineDarrazi/SmartExplorer
Installed-Size: $(du -sk "$STAGE/usr" | cut -f1)
Description: SmartExplorer - explorateur de fichiers multiplateforme
 Explorateur de fichiers leger en Rust (egui/eframe) avec vignettes,
 serveurs distants (SSH/SFTP, FTP), tri, recherche et vue proprietes
 systeme. Multiplateforme : macOS, Linux et Windows.
CONTROL

# 5. Construction + verification.
mkdir -p "$OUT_DIR"
DEB_FILE="$OUT_DIR/${PACKAGE}_${VERSION}_${ARCHITECTURE}.deb"
dpkg-deb --build --root-owner-group "$STAGE" "$DEB_FILE"
dpkg-deb --info "$DEB_FILE" >/dev/null
dpkg-deb --contents "$DEB_FILE" >/dev/null

echo ""
echo "============================================================"
echo " TERMINE : $DEB_FILE"
echo "============================================================"
ls -lh "$DEB_FILE"