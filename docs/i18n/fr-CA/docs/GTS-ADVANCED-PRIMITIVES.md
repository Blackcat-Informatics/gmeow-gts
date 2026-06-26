<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-ADVANCED-PRIMITIVES.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Contrat des primitives avancées GTS

> Traduction informative de [`docs/GTS-ADVANCED-PRIMITIVES.md`](../../../../docs/GTS-ADVANCED-PRIMITIVES.md). Le document anglais demeure la source normative pour les intégrations, les fonctionnalités avancées, les profils optionnels, les données de référence, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md) et reste informative.


Ce document rassemble le parcours d'implémentation pour les puits de diffusion en continu, les index, les MMR/preuves, le range-fetch, la réplication et les repères de mémoire. Le format filaire central reste normatif dans [`GTS-SPEC.md`](./GTS-SPEC.md) ; ce contrat énonce ce que les paquets actuels prennent réellement en charge et ce qui est intentionnellement différé (deferred) de la surface v1.
## Soutien actuel de la V1

| primitive | soutien actuel | limite de revendication |
|---|---|---|
| Propriété de repli de préfixe (Prefix-fold) | Chaque vecteur de corpus de haut niveau est testé aux limites des éléments CBOR. | Ceci prouve des lectures de préfixes totales, et non une API de récepteur de diffusion en continu (streaming sink). |
| Disposition diffusable en continu (Streamable layout) | `gts compact --streamable` réécrit l'ordre de livraison et ajoute un pied de page `index` ; les lecteurs (readers) valident la revendication et signalent les queues accrétives. | Il s'agit d'une fonctionnalité de disposition de profil/outil de validation (Validating Tool/Profile Layout). |
| Champs de pied de page d'index | Les rédacteurs (writers) émettent `count`, `head`, `off` et `ti` ; les rédacteurs Rust peuvent opter pour `mmr`, et les lecteurs (readers) Rust valident les racines `mmr` lorsqu'elles sont présentes. | L'accès aléatoire par lecteur complet (Full-reader) à partir de `off`/`ti` n'est pas encore revendiqué. |
| Preuve MMR JSON | Tous les moteurs vérifient le JSON de preuve détaché par rapport à `vectors/proofs/` ; Rust expose également `Writer::add_index_with_mmr`, valide la `index.mmr` facultative et implémente `gts prove`. | La vérification détachée est multi-moteur ; la création de preuves à partir de fichiers GTS indexés reste réservée à Rust. |
| Inventaire de réplication | Les quatre CLI exposent `gts heads`, `gts segments`, `gts missing` et `gts resume` pour la comparaison de tête lisible par machine et la reprise par plage d'octets. | Surface de réplication v1 partagée ; `resume` ne commence qu'après un identifiant de trame (frame) vérifié à une limite d'élément CBOR balayée. |
| Introspection de blob | `gts ls` répertorie les condensés (digests) de blobs adressés par contenu, les tailles et les types de médias. | La récupération de plage (Range fetch) nécessite toujours un index vérifié ou un balayage des limites. |
| Assistant de repère (benchmark) de mémoire | `scripts/bench_reader_memory.py` signale la matérialisation par lecteur complet (full-reader), une base de balayage de trames (frame-scan), les lignes Rust `read_to_sink_from_reader` et TypeScript browser `foldStreamToSink`. Go signale ses preuves d'allocation pour lecteur complet et récepteur de diffusion en continu (streaming-sink) sans matérialisation avec `go test ./reader -bench 'Benchmark(ReadFull\|ReadToSink)CorpusVector' -benchmem`. | Le balayage de trames (frame scan) n'est pas un repli (fold) de lecteur de diffusion en continu (Streaming Reader) ; les lignes Rust, TypeScript et Go sont des preuves de mémoire de récepteur (sink-memory) pour leurs API nommées. |

Le package Go actuel PEUT (MAY) revendiquer le niveau `Streaming Reader` pour
`reader.ReadToSink(ctx, io.Reader, reader.Options, sink)`. Le package Rust PEUT (MAY) revendiquer ce niveau pour
`read_to_sink_from_reader(reader, ReadOptions, sink)`. Le package TypeScript browser PEUT (MAY) revendiquer
ce niveau pour `foldStreamToSink(stream, options)`. `read_to_sink(&[u8], ...)` de Rust et
`foldStream(stream, options)`/`readStream(stream, options)` de TypeScript restent des assistants de compatibilité ou de
retour de graphe plutôt que les surfaces de revendication nommées. Rust demeure le seul package qui
PEUT (MAY) revendiquer la création de preuves MMR. Les quatre packages PEUVENT (MAY) revendiquer la vérification de preuve détachée pour le
jeu d'accessoires (fixture set) dans `vectors/proofs/` et les verbes d'inventaire de réplication partagés.
Python NE DEVRAIT PAS (SHOULD NOT) revendiquer les niveaux de récepteur (sink) ou de création de preuves pour le moment ; Go et TypeScript NE DEVRAIENT PAS (SHOULD NOT) revendiquer la
création de preuves pour le moment.
## Verbes CLI avancés différés

Les rangées ci-dessous, lorsqu'elles sont présentes, constituent un vocabulaire planifié plutôt que des commandes publiques actuelles. Le script de garde [`scripts/check_advanced_contract.py`](../../../../scripts/check_advanced_contract.py) échoue si l'un de ces verbes apparaît dans une surface de répartition du moteur ou dans la matrice de parité CLI publique avant que ce tableau ne soit mis à jour. Le tableau peut être vide lorsque chaque verbe CLI avancé actuellement planifié a été promu.

<!-- advanced-cli-deferred:start -->
| verb | status | next implementation gate |
|---|---|---|
<!-- advanced-cli-deferred:end -->
## Streaming Sink API

Un paquet PEUT (MAY) revendiquer `GTS Streaming Reader` uniquement lorsqu'il expose une API documentée qui replie (folds) ou projette en consommant les trames (frames) dans l'ordre et en émettant des événements vers un collecteur (sink) sans matérialiser l'intégralité du `Graph`.

Exigences minimales :

- vérifier l'id d'en-tête (header id) et l'id de trame/chaîne de précédence (frame id/prev chain) pendant la diffusion en continu ;
- conserver ou déverser (spill) le dictionnaire de termes au besoin, car les identifiants de termes sont locaux au segment ;
- émettre les événements term, quad, reifier, annotation, suppression, blob, opaque, signature, diagnostic, segment-head et streamable-layout dans l'ordre des trames ;
- enregistrer les mêmes diagnostics finaux et identifiants d'en-tête de segment que le lecteur (reader) complet pour la même entrée ;
- conserver une mémoire limitée par `O(distinct terms + maximum decoded frame size + validation sidecar state)`, et non par les triplets repliés (folded triples) ou les blobs ;
- signaler le comportement de la mémoire avec `scripts/bench_reader_memory.py` ou un repère (benchmark) équivalent.

Le sous-ensemble `streaming-property` existant demeure précieux, mais il s'agit d'une propriété de totalité par préfixe (prefix-totality). Il ne constitue pas en soi une revendication de collecteur de flux (streaming sink claim).
## Index, MMR et niveau de preuve

La charge utile facultative `index` comporte actuellement cinq éléments implémentés :

- `count` : nombre de trames couvertes ;
- `head` : identifiant de la dernière trame couverte ;
- `off` : décalage d'octets de chaque trame couverte depuis le début de son segment ;
- `ti` : mappage du type de trame vers les positions des trames couvertes ;
- `mmr` : racine Merkle-Mountain-Range réservée à Rust dans les fichiers GTS indexés sur les identifiants de trames couvertes.

Les éléments suivants restent différés :

- `dict` : localisateur de dictionnaire de termes utilisé par les projections de texte nécessitant une passe de dictionnaire ;
- création de preuves d'inclusion inter-moteurs à partir de fichiers GTS indexés ;
- verbes CLI `prove` en dehors de Rust.

Avant de promouvoir la création de preuves MMR au-delà de Rust, le dépôt nécessite :

- des fixtures de création de preuves pour fichiers indexés, incluant les comportements positifs et négatifs ;
- une implémentation de lecteur/rédacteur `index.mmr` en Python, Go et TypeScript par rapport aux préimages stables de `GTS-SPEC.md` ;
- des tests de création de preuves qui démontrent que le JSON détaché généré se vérifie indépendamment de la disponibilité du fichier complet dans chaque moteur.
## Règles de récupération par plage

La récupération par plage (range fetch) n'est précise à l'octet près qu'une fois que l'appelant dispose des limites de trames (frame boundaries).

Avec un tableau d'index `off` vérifié, le début de la trame `i` est :

```text
segment_start + off[i]
```

La fin de la trame `i` est la prochaine limite connue :

```text
segment_start + off[i + 1]       # when i + 1 is still covered
index_frame_start                # for the last covered frame, after a boundary scan
```

La charge utile de l'index actuel ne stocke pas les longueurs de trame. Par conséquent, un client NE DOIT PAS (MUST NOT) déduire la plage d'octets exacte de la dernière trame couverte à partir de `off` seul ; il doit connaître le début de la trame d'index à partir d'un balayage, de métadonnées de conteneur ou d'une future extension d'index portant la longueur.

Sans index, la récupération par plage est toujours possible, mais nécessite un balayage séquentiel des limites CBOR à partir du début du segment. Les requêtes HTTP `Range` ne sont alors sûres que pour les plages dont le début et la fin ont été dérivés des limites d'éléments balayés.
## Flux de travail de réplication

Toutes les CLI du moteur implémentent les verbes de réplication :

```bash
gts heads local.gts
gts segments local.gts
gts missing --from-head <peer-head> local.gts
gts resume --after <frame-id> local.gts
```

Les formes JSON Rust stables sont :

```text
gts-replication-heads-v1
gts-replication-segments-v1
gts-replication-missing-v1
```

Sémantique partagée :

- `heads` signale les têtes de segment dans l'ordre du fichier et une vue agrégée adaptée à la comparaison entre pairs ;
- `segments` signale la plage d'octets, le profil, la tête, le nombre de trames (frame count) et l'état de la disposition de chaque segment ;
- `missing` compare la tête connue d'un pair par rapport à l'ascendance locale des segments/trames et renvoie des plages d'octets exactes ou un résultat explicite « unknown; scan required » ;
- `resume` émet des octets seulement après avoir prouvé que l'identifiant de trame demandé existe et que la sortie commence à une limite d'élément CBOR.
## Repères de mémoire

La suite de repères de version couvre la lecture, le repli, l'écriture/depuis-N-Quads, l'empaquetage/dépaquetage files-profile, et les preuves de mémoire en continu à travers les moteurs qui exposent chaque surface :

```bash
just bench-release
```

La suite écrit du JSON lisible par machine et un rapport Markdown sous `dist/benchmarks/`. Utilisez [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md) comme modèle de note de version v1 ou d'annexe d'article.

Exécutez l'assistant local contre un ou plusieurs fichiers GTS :

```bash
cd python
uv run python ../scripts/bench_reader_memory.py ../vectors/25-streamable-source.gts
```

L'assistant émet quatre rangées par fichier lorsque Rust, Cargo, Node, npm, et les dépendances de construction TypeScript sont disponibles :

- `full-reader` : matérialise un `Graph` avec le lecteur Python actuel ;
- `frame-scan` : décode un élément CBOR à la fois et compte les en-têtes/trames sans repli ;
- `streaming-fold` : exécute l'assistant de repère de puits `read_to_sink_from_reader` de Rust et rapporte le RSS de haut niveau du processus Rust (`VmHWM`) sur Linux ;
- `typescript-streaming-fold` : exécute le chemin de puits `foldStreamToSink` du navigateur sous l'environnement d'exécution Web Streams de Node et rapporte le RSS de Node.

Les montages de régression d'exportation relationnelle Rust couvrent le chemin d'émission de rangées délimité : le chargeur de base de données diffuse du SQL dans `sqlite3`/`duckdb`, laisse les entrées de blobs paresseuses non mises en cache dans le graphe replié, et s'arrête avant `COMMIT` si un blob transformé ne peut pas être décodé. La contrainte de schéma restante est intentionnelle : les exportations `blobs.bytes` DOIVENT (MUST) tout de même décoder chaque charge utile en ligne de manière transitoire pour la rangée en cours d'écriture.

Les futures implémentations en continu DEVRAIENT (SHOULD) ajouter des repères de puits sans matérialisation spécifiques au moteur qui rapportent la mémoire de crête par termes distincts, la taille maximale de trame décodée, l'état de validation sidecar, les triplets et les tailles de blobs.
