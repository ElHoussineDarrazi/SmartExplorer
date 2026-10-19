# Politique de sécurité

## Périmètre

SmartExplorer stocke localement des identifiants de serveurs distants (SSH/SFTP et FTP), chiffrés avec XChaCha20-Poly1305 sous une clé dérivée d'un mot de passe maître (Argon2id). Toute faille touchant ce chiffrement, l'exécution de commandes, ou l'accès aux fichiers locaux est **sévérité haute**.

## Signaler une vulnérabilité

**Ne publiez pas de faille connue dans une issue publique.** Utilisez plutôt la fonctionnalité privée GitHub : [Security Advisory → Report a vulnerability](https://github.com/ElHoussineDarrazi/SmartExplorer/security/advisories/new).

Vous pouvez aussi contacter directement le mainteneur via GitHub (onglet profil). Merci de fournir :

- Une description claire de la faille et son impact
- Les étapes de reproduction (ou un proof-of-concept)
- La version concernée (`SmartExplorer → à propos`, ou le tag de la Release)

## Engagement du mainteneur

- Accusé de réception sous **72 h**
- Évaluation et mise à jour régulière de la situation
- Crédit du découvreur dans le bulletin de correction (sauf objection)
- Pas de poursuite pour les chercheurs de bonne foi qui signalent de façon responsable

## Corrections

Les correctifs sont publiés via une nouvelle Release (tag `v*`). Pour la dernière version, voir la page [Releases](https://github.com/ElHoussineDarrazi/SmartExplorer/releases).
