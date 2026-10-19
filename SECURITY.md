# Security Policy

## Scope

SmartExplorer stores remote server credentials (SSH/SFTP and FTP) locally, encrypted with XChaCha20-Poly1305 under a key derived from a master password (Argon2id). Any vulnerability affecting this encryption, command execution, or local file access is **high severity**.

## Reporting a vulnerability

**Do not disclose a known flaw in a public issue.** Instead, use GitHub's private feature: [Security Advisory → Report a vulnerability](https://github.com/ElHoussineDarrazi/SmartExplorer/security/advisories/new).

You may also contact the maintainer directly via GitHub (profile tab). Please provide:

- A clear description of the flaw and its impact
- Steps to reproduce (or a proof of concept)
- The affected version (`SmartExplorer → About`, or the Release tag)

## Maintainer commitment

- Acknowledgment within **72 hours**
- Assessment and regular status updates
- Credit to the discoverer in the advisory (unless declined)
- No legal action against good-faith researchers reporting responsibly

## Fixes

Fixes are published through a new Release (tag `v*`). For the latest version, see the [Releases](https://github.com/ElHoussineDarrazi/SmartExplorer/releases) page.
