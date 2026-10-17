---
name: exemple-skill
description: Génère et valide un rapport structuré à partir de fichiers ou d'un dossier. À utiliser lorsqu'on me demande d'analyser, résumer ou produire un rapport sur du contenu.
---

# Exemple de skill — rapport

## Quand l'utiliser
- Dès qu'on me demande d'analyser un fichier, de résumer du contenu ou de produire un rapport structuré.
- Pertinent pour des tâches répétitives de lecture, de synthèse et de validation de contenu.

## À éviter
- Pour compiler, lancer des tests ou corriger du code : cela relève d'outils dédiés, pas d'un rapport.

## Processus
1. Lire le fichier ou le dossier demandé en entier (ou un échantillon si très volumineux).
2. Extraire les éléments clés demandés : contexte, résultats, recommandations.
3. Structurer la réponse avec ces en-têtes : `## Contexte`, `## Résultats`, `## Recommandations`.
4. Aucune information chiffrée ne doit être inventée : toute valeur doit provenir du fichier lu ou être explicitement flagguée comme hypothèse.

## Fichiers liés
- [Documentation avancée](docs/avance.md)
- [Script de validation](scripts/helper.sh)
