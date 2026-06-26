<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-RELEASE-SLSA.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Posture de version SLSA

> Traduction informative de [`docs/GTS-RELEASE-SLSA.md`](../../../../docs/GTS-RELEASE-SLSA.md). Le document anglais demeure la source faisant autorité pour la gouvernance, la sécurité, les versions, les licences, la contribution, les obligations de conduite, les processus de divulgation et les commandes exécutables. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

## Décision

Différer la migration vers les flux de travail réutilisables pour le chemin de version v1.0-rc1. Les artefacts de version actuels devraient être décrits comme des preuves SLSA v1.0 Build Level 2 attestées par les artefacts GitHub, en plus du registre, du SBOM, de la version immuable et des contrôles de vérification publics décrits ci-dessous. Ne revendiquez pas le niveau SLSA v1.0 Build Level 3 pour les artefacts de version GTS tant que les couloirs de version ne sont pas déplacés vers des flux de travail réutilisables renforcés et que les artefacts représentatifs ne sont pas vérifiés par rapport à l'identité du flux de travail du signataire prévu.

Il s'agit d'une décision de documentation, et non d'une réduction du durcissement des versions. Les flux de travail de version actuels fournissent déjà des preuves publiques solides pour le modèle de version multilingue. Le déplacement de tous les couloirs vers des flux de travail réutilisables n'en vaut la peine que lorsque les flux de travail réutilisables créent une frontière de confiance plus claire et que la vérification par le consommateur peut faire respecter cette frontière.

## Base

GitHub documente les attestations d'artefact comme des preuves SLSA v1.0 Build Level 2.
GitHub documente également les flux de travail réutilisables (reusable workflows) comme la voie vers une isolation plus forte pour l'alignement SLSA v1.0 Build Level 3, car la construction peut être liée à des instructions de construction connues et vérifiées.

Les couloirs de version (release lanes) GTS actuels sont des fichiers de flux de travail de première partie dans ce dépôt, à l'exception de `visual-hashing`, qui est maintenant publié à partir de son dépôt autonome :

| Couloir de version (Release lane) | Flux de travail (Workflow) | Chemin de publication actuel |
|---|---|---|
| Rust `gmeow-gts` crate | `.github/workflows/release-cargo.yaml` | crates.io Trusted Publishing via GitHub Actions OIDC |
| Rust `gmeow-gts-capi` source crate | `.github/workflows/release-cargo-capi.yaml` | Jeton de bootstrap crates.io pour la première publication ; suivi Trusted Publishing requis |
| Rust `visual-hashing` crate | `Blackcat-Informatics/visual-hashing:.github/workflows/release.yml` | crates.io Trusted Publishing via GitHub Actions OIDC |
| Package Python | `.github/workflows/release-pypi.yml` | PyPI Trusted Publishing avec attestations de package |
| Package TypeScript | `.github/workflows/release-npm.yaml` | npm Trusted Publishing et provenance npm |
| Package Lua | `.github/workflows/release-luarocks.yaml` | Publication par jeton API LuaRocks ; aucune provenance native au registre |
| Package Ruby | `.github/workflows/release-rubygems.yaml` | RubyGems Trusted Publishing via GitHub Actions OIDC |
| Actifs CLI Go | `.github/workflows/release-go.yaml` | Actifs GitHub Release immuables |
| Actifs natifs C ABI | `.github/workflows/release-capi.yaml` | Archives GitHub Release immuables |

La refactorisation de ces tâches en flux de travail réutilisables (reusable workflows) au sein du même dépôt centraliserait la logique de version (release), mais cela n'ajouterait pas en soi une séparation de gouvernance suffisante pour justifier le changement de chaque couloir de version (release lane) immédiatement avant la v1.0-rc1. L'amélioration la plus forte est une frontière de flux de travail réutilisable (reusable-workflow) protégée et révisée, avec une vérification qui exige l'identité attendue du flux de travail réutilisable.

## Garanties actuelles

Chaque voie de version DOIT (MUST) conserver ces contrôles :

- vérifications de version tag-vers-manifeste avant publication ;
- permissions GitHub Actions de moindre privilège pour les tâches de version ;
- actions tierces épinglées ;
- OIDC de registre, provenance native du registre ou amorçage de jeton de première publication documenté là où c'est nécessaire ;
- attestations GitHub de provenance de construction pour les artefacts publiés ;
- attestations SPDX SBOM pour les artefacts de registre représentatifs et les archives Go ;
- GitHub Releases immuables pour Go et C ABI pour les archives, les sommes de contrôle et les actifs SBOM ;
- vérification publique après version (post-release) via `just verify-release`, avec une planification déterministe `just verify-release-dry-run` avant que les registres ne soient en ligne ;
- vérification après version du paquet d'enveloppement (wrapper) via `just verify-wrapper-release` lorsque les paquets d'enveloppement C ABI sont publiés, incluant des vérifications de liens de métadonnées de registre et des rapports d'état `published` / `pending` / `metadata-mismatch` / `missing` archivés.

La durabilité actuelle des preuves est :

| Surface | Artefact durable | Preuve d'attestation |
|---|---|---|
| Go | Archives de GitHub Release immuables, `checksums.txt` et `sbom-go-gts.spdx.json` | Attestation de version GitHub, attestations de provenance SLSA et attestations SPDX SBOM |
| C ABI | Archives de GitHub Release immuables, `checksums.txt` et `sbom-gmeow-gts-capi.spdx.json` | Attestation de version GitHub, attestations de provenance SLSA et attestations SPDX SBOM |
| crates.io `gmeow-gts` | Paquet `.crate` hébergé sur le registre | Provenance GitHub SLSA et attestations SPDX SBOM |
| crates.io `gmeow-gts-capi` | Paquet `.crate` hébergé sur le registre | Provenance GitHub SLSA et attestations SPDX SBOM ; jeton d'amorçage jusqu'à ce que le suivi Trusted Publishing soit intégré |
| PyPI | Wheel et sdist hébergés sur le registre | Attestations de publication PyPI plus provenance GitHub SLSA et attestations SPDX SBOM |
| npm | Tarball hébergé sur le registre | Provenance npm plus provenance GitHub SLSA et attestations SPDX SBOM |
| RubyGems | Paquet `.gem` hébergé sur le registre | Provenance GitHub SLSA et attestations SPDX SBOM |
| NuGet `Gmeow.Gts` | Paquet `.nupkg` hébergé sur le registre | Vérification des métadonnées du registre et du téléchargement du paquet ; aucune attestation de projet jusqu'à ce qu'une voie de version NuGet en ajoute une |
| Packagist `blackcatinformatics/gmeow-gts` | Métadonnées d'étiquette VCS de Packagist | Vérification des métadonnées du registre et de la référence source ; le contenu du paquet provient du commit de racine de paquet (package-root) étiqueté |
| LuaRocks `gmeow-gts` | Rockspec/source rock hébergé sur le registre | Vérification du manifeste du registre et du téléchargement du rockspec ; aucune attestation de projet dans la voie actuelle LuaRocks |
| Swift Package Index | Étiquette de version sémantique du dépôt et URL de paquet SPI | Vérification de l'étiquette Git et enregistrement d'URL SPI canonique ; la source du paquet est le dépôt étiqueté |
| r-universe `gmeowgts` | Paquet source R hébergé sur le registre | Index PACKAGES et vérification du téléchargement du tarball source ; aucune attestation de projet jusqu'à ce qu'une voie de version R en ajoute une |
| Julia General `GmeowGTS` | Métadonnées de paquet du registre General | Vérification de l'identité et de la version du registre ; la source du paquet est l'entrée du dépôt étiquetée |
| Conan/vcpkg | Essais à blanc (dry-runs) locaux de première partie | Aucune preuve de registre public jusqu'à ce que les recettes amont (upstream) soient acceptées |

## Orientation future vers Build Level 3

N'élevez la posture que lorsque le modèle de version (release) peut vérifier la limite plus stricte :

1. Créez des flux de travail (workflows) réutilisables pour les étapes de construction (build), d'empaquetage (package), de SBOM et d'attestation pour chaque couloir de version (release lane), ou un ensemble plus restreint d'usines de version (release factories) partagées lorsque les écosystèmes peuvent partager l'implémentation en toute sécurité.
2. Protégez ces flux de travail (workflows) réutilisables avec des règles de dépôt, une révision requise et CODEOWNERS. Privilégiez un dépôt de flux de travail (workflows) géré par l'organisation si le gain de gouvernance (governance) doit être plus fort que la protection de branche normale de ce dépôt.
3. Maintenez les flux de travail (workflows) appelants de petite taille. Les appelants DEVRAIENT (SHOULD) ne transmettre que la version, l'étiquette (tag) et les entrées release-material, puis laisser le flux de travail (workflow) réutilisable construire, empaqueter, générer les SBOM, attester et publier.
4. N'accordez aux flux de travail (workflows) appelants et réutilisables que les permissions requises pour le couloir, y compris `contents: read`, `id-token: write`, et `attestations: write` pour les tâches de génération d'attestations.
5. Préservez les chemins OIDC de registre/publication de confiance (trusted-publishing) existants, le flux de version (release flow) immuable de Go, la génération de SBOM et la vérification de fumée (smoke verification) `just verify-release` / `just verify-wrapper-release`.
6. Étendez `scripts/verify_release.py` avec des entrées facultatives de politique de signataire afin qu'une version (release) puisse exiger `gh attestation verify` avec `--signer-workflow <owner>/<repo>/.github/workflows/<workflow>.yml@<ref>` et, le cas échéant, `--signer-repo ...`.
7. Validez au moins un artefact représentatif de chaque couloir de version (release lane) adopté par rapport à l'identité attendue du flux de travail (workflow) réutilisable avant de revendiquer le niveau Build Level 3.

Tant que ces étapes ne sont pas terminées, les notes de version (release notes) et les listes de contrôle DEVRAIENT (SHOULD) indiquer la posture actuelle comme étant des attestations d'artefacts SLSA v1.0 Build Level 2 avec provenance de registre, attestations SBOM, versions (releases) Go immuables et couverture par vérificateur public.

## Références

- [Attestations d'artefact GitHub](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- [Utilisation des attestations d'artefact et des flux de travaux réutilisables pour atteindre le niveau de construction SLSA v1 3](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/increase-security-rating)
