# Contributing to SmartExplorer

Thank you for your interest! This project is a cross-platform file manager written in Rust, and any help is welcome. This document explains how to contribute properly.

## Table of contents

- [Code of Conduct](#code-of-conduct)
- [Getting started](#getting-started)
- [Branch & commit conventions](#branch--commit-conventions)
- [Submitting a change](#submitting-a-change)
- [Code style](#code-style)
- [Testing](#testing)
- [Adding a feature](#adding-a-feature)

## Code of Conduct

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to abide by it.

## Getting started

1. **Fork** the repository and clone your fork:

   ```bash
   git clone https://github.com/<your-username>/SmartExplorer.git
   cd SmartExplorer
   git remote add upstream https://github.com/ElHoussineDarrazi/SmartExplorer.git
   ```

2. **Install Rust** via [rustup](https://rustup.rs) (edition 2024 → Rust **1.85+**).

3. **System dependencies** per platform (required to build `ssh2`):

   - Linux: `sudo apt install libssh2-1-dev pkg-config`
   - macOS: `brew install libssh2`
   - Windows: none (OpenSSL is vendored)

4. **Verify everything builds**:

   ```bash
   cargo build
   cargo test
   ```

## Branch & commit conventions

- Base: always work on a branch created from `main`:
  `git checkout -b feature/my-feature` or `fix/my-fix`.
- Commits: message in **English or French**, prefixed with a type
  (following [Conventional Commits](https://www.conventionalcommits.org/)):

  | Prefix | Usage |
  |---|---|
  | `feat:` | new feature |
  | `fix:` | bug fix |
  | `docs:` | documentation |
  | `style:` | formatting, no behavior change |
  | `refactor:` | refactoring without behavior change |
  | `test:` | adding/fixing tests |
  | `chore:` | tooling, dependencies, CI |

  Examples: `feat(ui): icons view`, `fix(ssh): reconnect after network loss`.

## Submitting a change

1. **Sync** with upstream before starting:

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Commit** in logical steps, with clear messages.

3. **Check quality** (see [Code style](#code-style) and [Testing](#testing)):

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

4. **Open a Pull Request** against the upstream `main`:
   - Explicit title, prefixed by the type (`feat:`, `fix:`…)
   - Description: **what** and **why**; if fixing a bug, **how to reproduce**
   - Reference the related issue (`Fixes #123`) if relevant
   - One feature = one PR (avoid “grab-bag” PRs)

5. **CI** (fmt, clippy, build on 3 OS, tests) runs automatically on the PR.
   All checks must pass before review.

## Code style

- **`cargo fmt`**: the official Rust formatting is the reference (no house style).
- **`cargo clippy -D warnings`**: no warning tolerated. Read Clippy's suggestions.
- **Documentation**: every module, function, and public struct carries a doc-comment (`///`).
  Check with `cargo doc --no-deps --open`.
- **Comments in French** to match the style of the existing code.
- **No `unwrap()` in production code** outside tests — prefer error handling
  (`?`, `thiserror`). The code already handles typed errors: respect that model.

## Testing

```bash
cargo test                # unit tests
cargo clippy --all-targets -- -D warnings
```

- Add a test for every **bug fix** (the test reproduces the bug and fails without the fix).
- New features must be covered by tests or, failing that, precisely
  described in the PR for manual review.
- Tests must **never** hit the real network or depend on the user's
  filesystem (use `std::env::temp_dir()`, as in `src/credentials.rs`).

## Adding a feature

For a **non-trivial new feature**, first open an [issue](https://github.com/ElHoussineDarrazi/SmartExplorer/issues) to discuss it before writing code — this avoids parallel, misaligned work.

Contributions ideas welcome (look for issues labeled `good first issue` or `help wanted`):

- New themes / UI customization
- Support for additional remote protocols (S3, WebDAV…)
- Performance improvements (thumbnail rendering, indexing)
- Translations
- Tests and documentation

Thanks again for your contribution!
