<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: SECURITY.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Politique de sécurité de GTS

> Traduction informative de [`SECURITY.md`](../../../SECURITY.md). Le document anglais demeure la source faisant autorité pour la gouvernance, la sécurité, les versions, les licences, la contribution, les obligations de conduite, les processus de divulgation et les commandes exécutables. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md) et reste informative.

GTS (la spécification Graph Transport Substrate et ses quatre moteurs — Rust, Python, Go et TypeScript) est maintenu par Blackcat Informatics® Inc. Nous prenons les rapports de sécurité au sérieux et demandons que les vulnérabilités soient signalées de manière privée afin qu'elles puissent faire l'objet d'une enquête et être corrigées de manière responsable.

## Versions prises en charge

| Version | Pris en charge |
| --- | --- |
| `0.1.x` | Oui |
| `< 0.1` | Non |

## Signaler une vulnérabilité

N'ouvrez pas de ticket GitHub public pour une vulnérabilité de sécurité.

Plutôt :

- envoyez un courriel à <security@blackcatinformatics.ca>
- incluez `SECURITY` dans l'objet du message
- nommez le ou les moteurs concernés (Rust / Python / Go / TypeScript) et la ou les versions
- décrivez le problème, l'impact et les versions affectées
- fournissez les étapes de reproduction, un fichier de preuve de concept `.gts` ou des correctifs lorsque possible

La surface de sécurité la plus pertinente pour ce projet est **la lecture de données GTS non fiables et l'exécution de l'outillage `gts`**, par exemple :

- **Analyse (parsing)** — le décodage CBOR et la chaîne de transformation/codec empilable (décompression des charges utiles `gzip` / `zstd`, y compris les entrées d'épuisement des ressources / bombes de décompression) et la gestion des trames opaques/dégradées.
- **Intégrité et crypto** — la vérification de la chaîne BLAKE3, la signature/vérification COSE (§9.2) et le chiffrement COSE (§9.3) ; une vérification incorrecte qui accepte un journal falsifié ou tronqué.
- **Le profil fichiers** — l'écriture sur disque de `gts unpack`/`extract` ; la traversée de chemin, les liens symboliques ou les échappements de chemin absolu (les moteurs sont censés refuser ceux-ci).
- **Chaîne d'approvisionnement** — les paquets publiés sur crates.io, PyPI, npm et le proxy de module Go, ainsi que les flux de travail de version (release)/CI qui les produisent.

## À quoi s'attendre

- accusé de réception dans les 48 heures
- triage initial dans les 7 jours
- remédiation et divulgation (disclosure) coordonnées après validation

Les délais de résolution dépendent de la gravité, de l'exploitabilité et des contraintes de version (release), mais nous visons à traiter les problèmes confirmés aussi rapidement que possible. Un correctif qui affecte le format filaire ou un comportement partagé sera coordonné entre les quatre moteurs et le corpus de conformité (conformance corpus).

## Processus de divulgation responsable

1. Signaler le problème en privé.
2. Les responsables valident et trient le signalement.
3. Un correctif est développé, examiné et testé (sur les moteurs affectés).
4. Une version (release) ou un avis de sécurité est préparé.
5. La divulgation publique suit une fois que les utilisateurs ont eu un délai raisonnable pour effectuer la mise à jour.

## Mention des rapporteurs

Pour chaque rapport de vulnérabilité résolu au cours des 12 derniers mois, les notes de version publiques ou les avis de sécurité DEVRAIENT (SHOULD) créditer le rapporteur, à moins que le rapporteur ne demande l'anonymat ou un traitement privé. Si plusieurs rapporteurs ont contribué à un problème confirmé, créditez chaque rapporteur souhaitant une reconnaissance publique.

La mention du rapporteur DEVRAIT (SHOULD) utiliser le nom, l'identifiant ou l'organisation fournis par le rapporteur. Si un rapporteur demande l'anonymat, les notes de version et les avis de sécurité DEVRAIENT (SHOULD) indiquer que le problème a été signalé de manière privée sans le nommer.

Il n'y a actuellement aucune vulnérabilité de projet divulguée publiquement et résolue au cours des 12 derniers mois.

## Mises à jour de sécurité

- surveiller le dépôt pour les versions et les avis
- maintenir les dépendances à jour
- mettre à jour vers la dernière version prise en charge lorsque des correctifs sont publiés

## Contact

- Courriel : <security@blackcatinformatics.ca>
- Clé PGP pour les rapports chiffrés :

```text
-----BEGIN PGP PUBLIC KEY BLOCK-----
Version: GnuPG v2

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
=fCTf
-----END PGP PUBLIC KEY BLOCK-----
```

Pour les questions non liées à la sécurité ou les rapports de bogues, utilisez les canaux publics normaux d'issues ou de discussions.
