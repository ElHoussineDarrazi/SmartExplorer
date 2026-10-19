# SmartExplorer

[![CI](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/ci.yml/badge.svg)](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/ci.yml)
[![Build and Release](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/release.yml/badge.svg)](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://rust-lang.org)

Explorateur de fichiers multiplateforme (Windows, Linux, macOS) écrit en Rust, avec une interface 100 % Rust basée sur [egui/eframe](https://github.com/emilk/egui) — aucune dépendance système pour l'UI.

**Fonctionnalités** : navigation par onglets, vignettes d'images, vue Miniatures (avec aperçus PDF sur macOS), connexions distantes SSH/SFTP et FTP, stockage des identifiants chiffré (XChaCha20-Poly1305 + Argon2id), fenêtre Propriétés de l'ordinateur (OS, CPU, GPU, RAM, stockage), corbeille système native (Corbeille Windows / Poubelle macOS/Linux).

## Sommaire

- [Installation](#installation)
- [Compilation depuis les sources](#compilation-depuis-les-sources)
- [Intégration continue & releases](#intégration-continue--releases)
- [Contribuer](#contribuer)
- [Sécurité des identifiants](#sécurité-des-identifiants)
- [Licence](#licence)

---

## Installation

Téléchargez l'installateur de votre plateforme depuis la page [Releases](https://github.com/ElHoussineDarrazi/SmartExplorer/releases) :

| Plateforme | Fichier |
|---|---|
| 🍎 macOS | `SmartExplorer-macos.dmg` |
| 🪟 Windows | `SmartExplorer-windows-setup.exe` |
| 🐧 Linux (Debian/Ubuntu) | `smartexplorer_<version>_amd64.deb` |

### 🍎 macOS — contournement de Gatekeeper

L'application n'est **pas signée avec un Apple Developer ID ni notariée** (compte payant requis). Au premier lancement, macOS affiche donc :

> « Élement « SmartExplorer » non ouvert — Apple n'a pas pu confirmer que … »

C'est **normal** : le code est sûr, macOS se contente de ne pas pouvoir vérifier son origine. Pour lancer l'application, choisissez **une** de ces méthodes :

**Méthode 1 — Clic droit (la plus simple)**
1. Dans le Finder, ouvrez `/Applications`.
2. **Clic droit** sur **SmartExplorer** → **Ouvrir**.
3. Confirmez **Ouvrir** dans la fenêtre qui apparaît (celle-ci, contrairement au blocage initial, propose bien « Ouvrir »).

**Méthode 2 — Réglages Système**
1. Essayez d'ouvrir SmartExplorer une première fois (échoue, c'est normal).
2. Ouvrez **Réglages Système → Confidentialité et sécurité**.
3. Tout en bas, cliquez sur **« Ouvrir quand même »** à côté de SmartExplorer.

**Méthode 3 — Terminal (une fois pour toutes)**
```bash
xattr -dr com.apple.quarantine /Applications/SmartExplorer.app
```
L'application s'ouvrira ensuite normalement, comme toute app native.

> Ces réglages ne sont nécessaires qu'**au premier lancement** ; une fois contourné, SmartExplorer démarre directement.

### 🪟 Windows

Exécutez `SmartExplorer-windows-setup.exe` et suivez l'assistant. Si SmartScreen s'affiche (binaire non signé), cliquez sur **Informations complémentaires** puis **Exécuter quand même**.

### 🐧 Linux

```bash
sudo apt install ./smartexplorer_<version>_amd64.deb
```
Puis lancez **SmartExplorer** depuis votre menu d'applications, ou `smart-explorer` en terminal.

---

## Compilation depuis les sources

Prérequis : [Rust](https://rustup.rs) (outil `cargo`).

```bash
git clone https://github.com/ElHoussineDarrazi/SmartExplorer.git
cd SmartExplorer
cargo build --release
```

### Dépendances système par plateforme

- **Linux** : `libssh2-1-dev` (+ `pkg-config`) —
  `sudo apt install libssh2-1-dev pkg-config`
- **macOS** : `brew install libssh2`
- **Windows** : aucune — `ssh2` est compilé avec OpenSSL vendored (`openssl-src`), ce qui impose un Perl complet (Strawberry Perl) si vous reproduisez le build manuellement.

Le binaire release se trouve dans `target/release/` :

| Plateforme | Binaire |
|---|---|
| macOS | `smart_explorer` (ou `SmartExplorer.app` via `installer/macos/package.sh`) |
| Linux | `smart_explorer` |
| Windows | `smart_explorer.exe` |

> Note : `scripts/package_macos.sh` génère l'icône `.icns` à partir de `assets/icons/AppIcon.png` (nécessite `sips`, fourni par macOS).

---

## Intégration continue & releases

Chaque **tag** poussé (`v*`) déclenche le workflow [`.github/workflows/release.yml`](.github/workflows/release.yml), qui compile l'application sur les trois plateformes et publie automatiquement les installateurs (`.dmg`, `.deb`, `setup.exe`) sur la Release GitHub associée.

Pour couper une release :

```bash
# 1. Mets à jour `version` dans Cargo.toml, puis :
git commit -am "release: vX.Y.Z"

# 2. Crée le tag et pousse-le (la CI publie les installateurs) :
git tag -fa vX.Y.Z -m "SmartExplorer vX.Y.Z"
git push origin main vX.Y.Z
```

---

## Sécurité des identifiants

Les mots de passe des serveurs SSH/FTP ne sont **jamais stockés en clair** : ils sont chiffrés localement avec XChaCha20-Poly1305, la clé étant dérivée d'un mot de passe maître via Argon2id (sel aléatoire par enregistrement). Rien n'est envoyé sur le réseau — tout reste sur votre machine.

Pour signaler une faille de sécurité, suivez [SECURITY.md](SECURITY.md) (signalement privé, pas d'issue publique).

---

## Contribuer

Les contributions sont les bienvenues — que ce soit du code, de la documentation, des traductions ou des idées ! 🙌

**Nouveau ici ?** Voici le parcours typique pour un premier apport :

1. **Fourchez** le dépôt (bouton *Fork* en haut à droite).
2. Clonez **votre** fourche et créez une branche :

   ```bash
   git clone https://github.com/<votre-pseudo>/SmartExplorer.git
   cd SmartExplorer
   git checkout -b feature/ma-fonctionnalite
   ```

3. Modifiez le code, puis vérifiez la qualité (les mêmes contrôles que la CI) :

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

4. Poussez et ouvrez une **Pull Request**.

Le détail (style de code, convention de commits, tests) est dans **[CONTRIBUTING.md](CONTRIBUTING.md)**. En participant, vous acceptez le **[Code de conduite](CODE_OF_CONDUCT.md)**.

Chaque Pull Request est automatiquement validée par la [CI](.github/workflows/ci.yml) : formatage, lints stricts, tests et compilation sur les 3 plateformes.

**Idées d'amélioration** : consultez les issues étiquetées [`good first issue`](https://github.com/ElHoussineDarrazi/SmartExplorer/labels/good%20first%20issue) pour débuter en douceur.

---

## Licence

Distribué sous la licence **[MIT](LICENSE)** — réutilisez, modifiez, redistribuez librement, y compris dans des projets commerciaux.
