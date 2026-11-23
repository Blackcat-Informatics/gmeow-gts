<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-DUMP-DIR.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Répertoire de vidage GTS

> Traduction informative de [`docs/GTS-DUMP-DIR.md`](../../../../docs/GTS-DUMP-DIR.md). Le document anglais demeure la source normative pour les intégrations, les fonctionnalités avancées, les profils optionnels, les données de référence, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md) et reste informative.

`gts dump <archive.gts> --directory <out-dir>` développe une archive GTS dans un répertoire d'inspection versionné. La première implémentation est uniquement Rust ; la disposition est intentionnellement neutre vis-à-vis du langage afin que d'autres moteurs puissent suivre le même contrat plus tard.

Le vidage est une surface d'exploration et de diagnostic, et non un nouveau format de transfert. Il duplique des vues utiles de l'archive tout en évitant par défaut la duplication des octets de charge utile volumineux.

## Disposition

```text
out/
├── README.md
├── .gts-dump/
│   ├── manifest.json
│   ├── heads.json
│   └── segments.json
├── graph/
│   ├── README.md
│   ├── folded.nq
│   └── tables/
│       ├── terms.jsonl
│       ├── quads.jsonl
│       ├── reifiers.jsonl
│       ├── annotations.jsonl
│       ├── meta.jsonl
│       ├── blob-meta.jsonl
│       ├── suppressions.jsonl
│       ├── opaque.jsonl
│       ├── signatures.jsonl
│       └── diagnostics.jsonl
├── frames/
│   ├── README.md
│   ├── inventory.jsonl
│   └── segments/
│       └── 0000/
│           ├── header.json
│           ├── folded.nq
│           ├── frame-0001.nq
│           └── *.jsonl
├── blobs/
│   ├── index.jsonl
│   └── by-digest/
│       └── blake3/
└── files/
    ├── entries.jsonl
    └── tree/
```

Les répertoires sont omis lorsqu'il n'y a pas de contenu d'archive correspondant. Par
exemple, `files/` n'est présent que lorsque l'archive contient un catalogue de profil de
fichiers valide, et `blobs/by-digest/` n'est présent que lorsque le vidage doit stocker
des octets de charge utile blob qui ne sont pas déjà matérialisés via `files/tree/`.

## Vues de graphe

`graph/folded.nq` est la projection textuelle RDF faisant autorité pour l'archive repliée. N-Quads est le format par défaut car il est déterministe, orienté ligne et peut représenter des graphes nommés. Turtle n'est pas émis par défaut car il ne peut pas représenter l'ensemble du jeu de données RDF replié sans choix de politique ; TriG est un meilleur format explicite futur pour les utilisateurs qui souhaitent une syntaxe de jeu de données RDF plus lisible.

`graph/tables/*.jsonl` expose le même état replié sous forme de simples tableaux orientés ligne. Ceux-ci sont destinés aux outils shell, aux tableurs, à DuckDB, aux carnets Python et aux utilisateurs qui ne souhaitent pas comprendre la sérialisation RDF avant d'inspecter l'archive.

## Trames dépliées

`frames/inventory.jsonl` enregistre les plages d'octets de segment et de trame, les identifiants de trame, les types de trame et la validité. Chaque répertoire `frames/segments/NNNN/` contient les N-Quads repliés par segment et les lignes JSONL décodées au niveau de la trame. Les fichiers `frame-*.nq` sont émis lorsqu'une trame possède des contributions RDF pouvant être projetées sous forme de N-Quads.

La vue des trames dépliées répond à une question différente de `graph/` : elle montre ce que le journal d'ajout a contribué dans l'ordre, tandis que `graph/` montre l'état replié final.

## Politique relative aux charges utiles

Par défaut, la matérialisation de la charge utile s'effectue par copie unique :

- les charges utiles files-profile sont écrites sous `files/tree/` ;
- `blobs/index.jsonl` enregistre toujours digest, size, media type, suppression state et les chemins matérialisés ;
- `blobs/by-digest/` n'est écrit que pour les charges utiles blob qui ne sont pas déjà matérialisées via `files/tree/` ;
- les charges utiles supprimées sont indexées mais non matérialisées à moins que `--include-suppressed` ne soit transmis ;
- `--metadata-only` écrit les fichiers de graphe, de trame (frame), de manifeste et d'index sans extraire les octets de la charge utile.

Le dump ne copie pas le fichier original `.gts` par défaut. Le chemin source, size et digest sont enregistrés dans `.gts-dump/manifest.json`.

## Importation future

Le nom du schéma dans `.gts-dump/manifest.json` est `gts-dump-v1`. La future prise en charge de `undump` devrait (SHOULD) traiter le manifeste et le mappage de charge utile matérialisé comme le contrat d'importation. La commande Rust actuelle est unidirectionnelle : elle prépare la forme du répertoire pour une édition aller-retour sans revendiquer la prise en charge de l'importation pour le moment.
