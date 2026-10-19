# Contribuer à SmartExplorer

Merci de votre intérêt ! Ce projet est un explorateur de fichiers multiplateforme écrit en Rust, et toute aide est bienvenue. Ce document explique comment bien contribuer.

## Table des matières

- [Code de conduite](#code-de-conduite)
- [Démarrer](#démarrer)
- [Convention de branches et de commits](#convention-de-branches-et-de-commits)
- [Soumettre un changement](#soumettre-un-changement)
- [Style de code](#style-de-code)
- [Tester](#tester)
- [Ajouter une fonctionnalité](#ajouter-une-fonctionnalité)

## Code de conduite

Ce projet suit le [Code de conduite](CODE_OF_CONDUCT.md). En participant, vous vous engagez à le respecter.

## Démarrer

1. **Fourche** le dépôt (*fork*) et clonez votre fourche :

   ```bash
   git clone https://github.com/<votre-pseudo>/SmartExplorer.git
   cd SmartExplorer
   git remote add upstream https://github.com/ElHoussineDarrazi/SmartExplorer.git
   ```

2. **Installez Rust** via [rustup](https://rustup.rs) (édition 2024 → Rust **1.85+**).

3. **Dépendances système** par plateforme (nécessaires pour compiler `ssh2`) :

   - Linux : `sudo apt install libssh2-1-dev pkg-config`
   - macOS : `brew install libssh2`
   - Windows : aucune (OpenSSL compilé en vendored)

4. **Vérifiez que tout compile** :

   ```bash
   cargo build
   cargo test
   ```

## Convention de branches et de-commits

- Base : travaillez toujours sur une branche créée depuis `main` :
  `git checkout -b feature/ma-fonctionnalite` ou `fix/mon-correction`.
- Commits : message en **anglais ou français**, préfixé par un type
  (conforme [Conventional Commits](https://www.conventionalcommits.org/)) :

  | Préfixe | Usage |
  |---|---|
  | `feat:` | nouvelle fonctionnalité |
  | `fix:` | correction de bug |
  | `docs:` | documentation |
  | `style:` | formatage, sans changement de sens |
  | `refactor:` | refactorisation sans changement de comportement |
  | `test:` | ajout/correction de tests |
  | `chore:` | outillage, dépendances, CI |

  Exemples : `feat(ui): vue Miniatures`, `fix(ssh): reconnexion après perte réseau`.

## Soumettre un changement

1. **Sync** avec l'amont avant de commencer :

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Committez** par pas logiques, message clair.

3. **Vérifiez la qualité** (voir [Style de code](#style-de-code) et [Tester](#tester)) :

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

4. **Ouvrez une Pull Request** vers `main` du dépôt amont :
   - Titre explicite, préfixé par le type (`feat:`, `fix:`…)
   - Description : **quoi** et **pourquoi** ; si corrections un bug, **comment reproduire**
   - Référencez l'issue liée (`Fixes #123`) si pertinent
   - Une fonctionnalité = une PR (évitez les PRs « fourre-tout »)

5. La **CI** (fmt, clippy, build 3 OS, tests) s'exécute automatiquement sur la PR.
   Tous les checks doivent passer avant la revue.

## Style de code

- **`cargo fmt`** : le formatage officiel Rust est la référence (pas de style maison).
- **`cargo clippy -D warnings`** : aucun avertissement toléré. Lisez les suggestions de Clippy.
- **Documentation** : chaque module, fonction et struct publique porte une doc-comment (`///`).
  Vérifiez avec `cargo doc --no-deps --open`.
- **Commentaires en français** pour coller au style du code existant.
- **Pas de `unwrap()` dans le code de production** hors tests — préférez la gestion d'erreur
  (`?`, `thiserror`). Le code gère déjà des erreurs typées : respectez ce modèle.

## Tester

```bash
cargo test                # tests unitaires
cargo clippy --all-targets -- -D warnings
```

- Ajoutez un test pour chaque **correctif de bug** (le test reproduit le bug, échoue sans le fix).
- Les nouvelles fonctionnalités doivent être couvertes par des tests ou, à défaut,
  décrites précisément dans la PR pour la revue manuelle.
- Les tests ne doivent **jamais** accéder au réseau réel ni dépendre du système de fichiers
  de l'utilisateur (utilisez `std::env::temp_dir()`, comme dans `src/credentials.rs`).

## Ajouter une fonctionnalité

Pour une **nouvelle fonctionnalité non triviale**, ouvrez d'abord une [issue](https://github.com/ElHoussineDarrazi/SmartExplorer/issues) pour en discuter avant d'écrire le code — cela évite un travail parallèle non aligné.

Idées de contributions bienvenues (cherchez les issues étiquetées `good first issue` ou `help wanted`) :

- Nouveaux thèmes / personnalisation de l'UI
- Support de protocoles distants supplémentaires (S3, WebDAV…)
- Amélioration des performances (rendu des vignettes, indexation)
- Traductions
- Tests et documentation

Merci encore pour votre contribution !
