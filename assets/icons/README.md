# Icônes de l'application

## Fichiers

- `AppIcon.png` — **icône source** (recommandé : carré, ≥ 512×512 px). Utilisée automatiquement par le packaging.
- `AppIcon.icns` (optionnel) — si présent, il sera utilisé tel quel (prioritaire sur le PNG).

## Comment ça marche

`scripts/package_macos.sh` convertit automatiquement `AppIcon.png` en `.icns`
(via `sips` + `iconutil`, outils natifs macOS) et l'installe dans
`SmartExplorer.app/Contents/Resources/AppIcon.icns`.

Pour forcer une icône `.icns` spécifique :

```bash
ICON_ICNS=chemin/vers/ma.icns ./scripts/package_macos.sh
```

## Générer un `.icns` manuellement (optionnel)

À partir de `AppIcon.png` :

```bash
mkdir -p AppIcon.iconset
for size in 16 32 64 128 256 512 1024; do
  sips -z $size $size AppIcon.png --out AppIcon.iconset/icon_${size}x${size}.png
done
iconutil -c icns AppIcon.iconset
```