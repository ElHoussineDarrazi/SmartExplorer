#!/usr/bin/env bash
#
# package_macos.sh — Construit SmartExplorer pour macOS et produit un .dmg
# utilisable (glissez-déposez dans /Applications).
#
# Prérequis : rust (cargo), hdiutil (macOS).
#
# Variables d'environnement (toutes optionnelles) :
#   APP_NAME        Nom de l'app (défaut : SmartExplorer)
#   CARGO_NAME      Nom du binaire cargo (défaut : smart_explorer)
#   VERSION         Version affichée (défaut : lue de Cargo.toml)
#   IDENTIFIANT     Bundle ID (défaut : com.example.smartexplorer)
#   RELEASE         "1" pour un build release (défaut : 1)
#   ICON_ICNS       Chemin vers une icône .icns (défaut : icône egui par défaut)
#   OUT_DIR         Dossier de sortie (défaut : dist/)
#
set -euo pipefail

APP_NAME="${APP_NAME:-SmartExplorer}"
CARGO_NAME="${CARGO_NAME:-smart_explorer}"
IDENTIFIANT="${IDENTIFIANT:-com.smartexplorer.app}"
RELEASE="${RELEASE:-1}"
OUT_DIR="${OUT_DIR:-dist}"

# Lecture de la version dans Cargo.toml si non fournie.
if [[ -z "${VERSION:-}" ]]; then
  VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/' || true)"
fi
VERSION="${VERSION:-0.1.0}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BUILD_PROFILE="release"
if [[ "$RELEASE" != "1" ]]; then
  BUILD_PROFILE="debug"
fi

APP_BUNDLE="$APP_NAME.app"
APP_BIN="$CARGO_NAME"
TARGET_BIN="target/$BUILD_PROFILE/$APP_BIN"
DMG_NAME="SmartExplorer-macos.dmg"
STAGING="dmg_staging"

echo "============================================================"
echo " Packaging SmartExplorer pour macOS"
 echo " App        : $APP_NAME"
 echo " Version    : $VERSION"
 echo " Bundle ID  : $IDENTIFIANT"
 echo " Profil     : $BUILD_PROFILE"
 echo " Sortie     : $OUT_DIR/$DMG_NAME"
 echo "============================================================"

# 1. Build
echo "[1/5] Build cargo ($BUILD_PROFILE)…"
if [[ "$BUILD_PROFILE" == "release" ]]; then
  cargo build --release
else
  cargo build
fi

if [[ ! -x "$TARGET_BIN" ]]; then
  echo "ERREUR : binaire introuvable : $TARGET_BIN" >&2
  exit 1
fi

# 2. Structure du bundle .app
echo "[2/5] Création du bundle .app…"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_RESOURCES="$APP_CONTENTS/Resources"

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_MACOS" "$APP_RESOURCES"

# Copie du binaire exécutable.
cp "$TARGET_BIN" "$APP_MACOS/$APP_BIN"
chmod +x "$APP_MACOS/$APP_BIN"

# Icône : si une .icns est fournie, on l'utilise ; sinon on convertit AppIcon.png en .icns.
ICON_DEST="$APP_RESOURCES/AppIcon.icns"
ICON_PNG="${ICON_PNG:-assets/icons/AppIcon.png}"
if [[ -n "${ICON_ICNS:-}" && -f "$ICON_ICNS" ]]; then
  cp "$ICON_ICNS" "$ICON_DEST"
  echo "    Icône (.icns fournie) : $ICON_ICNS"
elif [[ -f "$ICON_PNG" ]]; then
  echo "    Conversion de l'icône PNG → .icns : $ICON_PNG"
  ICONSET="$(mktemp -d)/AppIcon.iconset"
  mkdir -p "$ICONSET"
  # (nom, taille en px) — sips permet l'upscaling pour les tailles > source.
  declare -a SIZES=(
    "16x16 16"
    "16x16@2x 32"
    "32x32 32"
    "32x32@2x 64"
    "128x128 128"
    "128x128@2x 256"
    "256x256 256"
    "256x256@2x 512"
    "512x512 512"
    "512x512@2x 1024"
  )
  for entry in "${SIZES[@]}"; do
    name="${entry%% *}"
    px="${entry##* }"
    sips -z "$px" "$px" "$ICON_PNG" --out "$ICONSET/icon_${name}.png" >/dev/null
  done
  iconutil -c icns "$ICONSET" -o "$ICON_DEST"
  rm -rf "$(dirname "$ICONSET")"
  echo "    Icône installée : $ICON_DEST"
else
  echo "    (aucune icône trouvée — icône par défaut du système)"
fi

# 3. Info.plist
echo "[3/5] Écriture de Info.plist…"
cat > "$APP_CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>$APP_NAME</string>
  <key>CFBundleExecutable</key>
  <string>$APP_BIN</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>$IDENTIFIANT</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSHighResolutionCapable</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

# 4. Nettoyage + staging pour le DMG
echo "[4/5] Préparation du staging DMG…"
rm -rf "$STAGING" "$DMG_NAME"
mkdir -p "$STAGING"
cp -R "$APP_BUNDLE" "$STAGING/$APP_BUNDLE"
# Lien pratique vers /Applications (style macOS classique).
ln -s /Applications "$STAGING/Applications"

# 5. Création du DMG
echo "[5/5] Création du .dmg…"
mkdir -p "$OUT_DIR"
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$STAGING" \
  -ov \
  -format UDZO \
  -o "$OUT_DIR/$DMG_NAME"

# Nettoyage du staging.
rm -rf "$STAGING"

echo ""
echo "============================================================"
echo " TERMINÉ : $OUT_DIR/$DMG_NAME"
echo "============================================================"
ls -lh "$OUT_DIR/$DMG_NAME"
