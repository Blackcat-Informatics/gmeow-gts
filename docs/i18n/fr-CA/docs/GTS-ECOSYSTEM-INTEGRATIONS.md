<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-ECOSYSTEM-INTEGRATIONS.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Contrat d'intégration de l'écosystème GTS

> Traduction informative de [`docs/GTS-ECOSYSTEM-INTEGRATIONS.md`](../../../../docs/GTS-ECOSYSTEM-INTEGRATIONS.md). Le document anglais demeure la source normative pour les intégrations, les fonctionnalités avancées, les profils optionnels, les données de référence, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md) et reste informative.

Ce document est le contrat public pour l'utilisation de GTS avec les bibliothèques RDF, les trames de données, les navigateurs, les services et les magasins d'objets. Le format filaire de base reste normatif dans [GTS-SPEC.md](./GTS-SPEC.md) ; ce document consigne ce que les moteurs actuels exposent, quels exemples sont pris en charge et ce qui est explicitement différé.

## Matrice d'état

| Écosystème | Chemin d'intégration actuel | Différés |
|---|---|---|
| Rust RDF | `gmeow_gts::nquads::to_nquads(&graph)` et `gmeow_gts::from_nquads::from_nquads(text)` demeurent le pont sans dépendance supplémentaire pour les crates RDF externes ; `--features rdf` active `gmeow_gts::rdf::{to_rdf_dataset, from_rdf_dataset}` pour une intégration `Dataset` native sans dépendance et sans magasin de graphes intégré ; `--features native-store` active `gmeow_gts::native_store::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}` et `Writer::from_store` en utilisant le magasin RDF en mémoire natif déterministe ; `--features rdf-codecs` active les codecs texte natifs N-Triples, Turtle, TriG et RDF/XML ; `gmeow_gts::examples::agent_memory` démontre une forme d'application en aval sans dépendances supplémentaires ; `gts to-sqlite` exporte par défaut le modèle de table d'entiers replié (folded), tandis que `to-duckdb` et `to-parquet` sont derrière la fonctionnalité Cargo sans dépendance `duckdb`. | Rio demeure différé (deferred) car la crate `rio_api` actuelle est marquée comme non maintenue en amont ; l'interopérabilité externe Sophia/Oxigraph/Rio utilise le pont texte N-Quads sans dépendance plutôt qu'un adaptateur interne à la crate. |
| RDF/données Python | `gts.from_rdflib()` et `gts.to_rdflib()` couvrent l'intégration rdflib RDF 1.1 `Graph`/`Dataset` ; `gts to-sqlite`, `to-duckdb` et `to-parquet` couvrent le transfert relationnel/data-frame. | L'exportation de triplets cités (quoted-triple) RDF 1.2 vers rdflib est stricte par défaut (strict-by-default) et avec perte (lossy) uniquement sur demande explicite. |
| Navigateur TypeScript | `@blackcatinformatics/gmeow-gts/browser` expose `foldStreamToSink(ReadableStream<Uint8Array>, options)` pour le niveau de Lecteur (Reader) en continu sans matérialisation, ainsi que `foldStream`, `readStream`, `toNQuads` retournant des graphes, des événements de repli (fold) progressifs et des points d'ancrage (hooks) de fournisseur de clés COSE Sign1/Encrypt0 basés sur WebCrypto. Le paquet racine porte également une condition de navigateur qui se résout en cette surface plus étroite pour les bundlers. | Le CLI Node uniquement et les assistants de système de fichiers `pack`/`unpack`/`diff` demeurent en dehors de l'exportation pour navigateur. La récupération par plage (range fetch) nécessite encore un index vérifié ou un balayage de limites (boundary scan). |
| Services Go | `reader.ReadFrom(ctx, io.Reader, reader.Options)` fournit l'intégration de service retournant un graphe, tandis que `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)` fournit des événements de repli (fold) en continu limités en octets et sensibles à l'annulation pour les corps HTTP, les objets de stockage d'objets et les tubes (pipes) ; le CLI Go expose également les verbes d'inventaire de réplication partagés. | L'orchestration de réplication spécifique au service demeure du code d'application construit sur les verbes partagés. |
| Wrappers ABI C | `rust/capi/` construit `libgts` et `rust/capi/include/gts.h` pour les environnements d'exécution compatibles C. Les wrappers C++, .NET, PHP, Lua, Swift, Ruby, R et Julia exposent les métadonnées ABI, la lecture/vérification de rapports JSON, la conversion de texte RDF pilotée par le registre, les assistants de profil de fichiers et les erreurs structurées tout en copiant les tampons (buffers) natifs dans des valeurs appartenant à l'écosystème. | Ces wrappers déléguent au moteur Rust et ne sont pas des moteurs de parité indépendants ou de nouvelles colonnes de CLI. Les archives `libgts` natives installables sont publiées via le canal GitHub Release `capi-v*` ; l'automatisation des sorties du registre de wrappers demeure séparée des canaux de sortie actuels des moteurs Rust/Python/Go/TypeScript. |
| Archives compatibles Tar | Les commandes Rust `gts from-tar`, `gts to-tar` et `gts tar -c/-x/-t/-d` sont disponibles derrière `--features tar`. Elles relient les flux (streams) `.tar`, `.tar.gz` et `.tar.zst` aux archives GTS files-profile-v2 avec des corps de fichiers adressés par condensé (digest-addressed), des métadonnées équivalentes à tar, la préservation des enregistrements PAX inconnus et des options d'extraction explicites (opt-ins). | La parité Python/Go/TypeScript est intentionnellement différée (deferred). Ces moteurs devraient implémenter l'importation/exportation files-profile-v2 et réussir `vectors/tar/` avant que leurs CLI ne revendiquent `from-tar`, `to-tar` ou `tar`. |
| Lots (bundles) OKF | Les commandes Rust `gts from-okf` et `gts to-okf` sont disponibles derrière `--features okf`. Elles transforment les lots (bundles) OKF Markdown + YAML-frontmatter en paquets de profil (profile) GTS `okf` et projettent les graphes de profil OKF en retour vers les répertoires de lots. Le corpus de conformité (conformance corpus) engagé inclut un lot de style BigQuery sous `vectors/okf/bigquery-join/`, incluant des pages de navigation `index.md` sans frontmatter correspondant aux échantillons Knowledge Catalog de Google. | La parité Python/Go/TypeScript est intentionnellement différée (deferred). Ces moteurs devraient implémenter le même contrat de répertoire `gts-okf-v1` et réussir le corpus OKF avant que leurs CLI ne revendiquent `from-okf` ou `to-okf`. |

## Contrat de wrapper ABI C

La politique de compatibilité de l'ABI C se trouve dans
[`rust/capi/README.md#compatibility-policy`](../rust/capi/README.md#compatibility-policy).
`GTS_ABI_VERSION` régit la frontière native `gts.h`/`libgts` et est distincte
des versions de paquets et des versions de schéma de rapport JSON. Les paquets de
wrappers DOIVENT (MUST) rejeter les versions d'ABI non prises en charge avec une
erreur de wrapper, une exception ou un échec d'installation/configuration clair
plutôt que de continuer silencieusement avec un contrat natif inconnu.

Les assistants (helpers) de chemin de profil de fichiers utilisent le contrat de
chemin de chaîne C UTF-8 terminée par NUL de l'ABI v1. La documentation des
wrappers NE DOIT PAS (MUST NOT) présenter ces assistants comme une couverture
complète des chemins de caractères larges Windows ; les futures fonctions de chemin
de caractères larges devraient être de nouveaux symboles d'ABI C additifs dans le
cadre de la politique de compatibilité.

| Écosystème | Chemin d'intégration actuel | Différés |
|---|---|---|
| Rust RDF | `gmeow_gts::nquads::to_nquads(&graph)` et `gmeow_gts::from_nquads::from_nquads(text)` demeurent le pont sans dépendance supplémentaire pour les caisses RDF externes ; `--features rdf` active `gmeow_gts::rdf::{to_rdf_dataset, from_rdf_dataset}` pour l'interopérabilité native `Dataset` sans dépendance et sans magasin de graphes intégré ; `--features native-store` active `gmeow_gts::native_store::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}` et `Writer::from_store` à l'aide du magasin RDF en mémoire natif déterministe ; `--features rdf-codecs` active les codecs texte natifs N-Triples, Turtle, TriG et RDF/XML ; `gmeow_gts::examples::agent_memory` démontre une forme d'application en aval sans dépendances supplémentaires ; `gts to-sqlite` exporte le modèle de table d'entiers repliés par défaut, tandis que `to-duckdb` et `to-parquet` sont derrière la fonctionnalité Cargo sans dépendance `duckdb`. | Rio demeure différé parce que la caisse `rio_api` actuelle est marquée comme non maintenue en amont ; l'interopérabilité externe Sophia/Oxigraph/Rio utilise le pont texte N-Quads sans dépendance plutôt qu'un adaptateur interne à la caisse. |
| Python RDF/données | `gts.from_rdflib()` et `gts.to_rdflib()` couvrent l'interopérabilité rdflib RDF 1.1 `Graph`/`Dataset` ; `gts to-sqlite`, `to-duckdb` et `to-parquet` couvrent le transfert relationnel/cadre de données. | L'exportation de triplets cités RDF 1.2 vers rdflib est stricte par défaut et n'entraîne de perte que si explicitement demandé. |
| TypeScript navigateur | `@blackcatinformatics/gmeow-gts/browser` expose `foldStreamToSink(ReadableStream<Uint8Array>, options)` pour le niveau Lecteur en continu (Streaming Reader) sans matérialisation, plus `foldStream`, `readStream`, `toNQuads` retournant des graphes, des événements de repli progressifs et des crochets de fournisseur de clés COSE Sign1/Encrypt0 basés sur WebCrypto. La racine du paquet porte également une condition de navigateur qui se résout en cette surface plus étroite pour les outils de regroupement. | Les assistants CLI et de système de fichiers `pack`/`unpack`/`diff` réservés à Node demeurent à l'extérieur de l'exportation pour navigateur. La récupération par plage (range fetch) nécessite encore un index vérifié ou un balayage des limites. |
| Go services | `reader.ReadFrom(ctx, io.Reader, reader.Options)` fournit l'intégration de service retournant des graphes, tandis que `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)` fournit des événements de repli en continu limités en octets et sensibles à l'annulation pour les corps HTTP, les objets de stockage d'objets et les tubes ; le CLI Go expose également les verbes d'inventaire de réplication partagés. | L'orchestration de réplication spécifique au service demeure du code d'application construit sur les verbes partagés. |
| Enveloppes C ABI | `rust/capi/` construit `libgts` et `rust/capi/include/gts.h` pour les environnements d'exécution compatibles C. Les enveloppes C++, .NET, PHP, Lua, Swift, Ruby, R et Julia exposent les métadonnées ABI, lisent/vérifient les rapports JSON, la conversion de texte RDF pilotée par registre, les assistants de profil de fichiers et les erreurs structurées tout en copiant les tampons natifs dans les valeurs appartenant à l'écosystème. | Ces enveloppes délèguent au moteur Rust et ne sont pas des moteurs de parité indépendants ou de nouvelles colonnes CLI. Les archives natives installables `libgts` sont publiées via la voie GitHub Release `capi-v*` ; l'automatisation de la publication du registre d'enveloppes demeure distincte des voies de publication actuelles des moteurs Rust/Python/Go/TypeScript. |
| Archives compatibles Tar | Les implémentations Rust `gts from-tar`, `gts to-tar` et `gts tar -c/-x/-t/-d` sont disponibles derrière `--features tar`. Elles relient les flux `.tar`, `.tar.gz` et `.tar.zst` aux archives GTS files-profile-v2 avec des corps de fichiers adressés par condensé, des métadonnées équivalentes à tar, la préservation des PAX inconnus et des options d'extraction explicites. | La parité Python/Go/TypeScript est intentionnellement différée. Ces moteurs devraient implémenter l'importation/exportation files-profile-v2 et passer `vectors/tar/` avant que leurs CLI ne revendiquent `from-tar`, `to-tar` ou `tar`. |
| Ensembles OKF | Les implémentations Rust `gts from-okf` et `gts to-okf` sont disponibles derrière `--features okf`. Elles transforment les ensembles OKF Markdown + frontmatter YAML en paquets de profil GTS `okf` et les graphes de profil OKF de projet en répertoires d'ensembles. Le corpus commis inclut un ensemble de style BigQuery sous `vectors/okf/bigquery-join/`, incluant des pages de navigation sans frontmatter `index.md` correspondant aux échantillons Knowledge Catalog de Google enregistrés. | La parité Python/Go/TypeScript est intentionnellement différée. Ces moteurs devraient implémenter le même contrat de répertoire `gts-okf-v1` et passer le corpus OKF avant que leurs CLI ne revendiquent `from-okf` ou `to-okf`. |

## Contrat d'enveloppe C ABI

La politique de compatibilité C ABI se trouve dans
[`rust/capi/README.md#compatibility-policy`](../rust/capi/README.md#compatibility-policy).
`GTS_ABI_VERSION` régit la limite native `gts.h`/`libgts` et est distincte
des versions de paquets et des versions de schéma de rapport JSON. Les paquets
d'enveloppe DOIVENT (MUST) rejeter les versions ABI non prises en charge avec
une erreur d'enveloppe claire, une exception ou un échec d'installation/configuration
plutôt que de continuer silencieusement avec un contrat natif inconnu.

Les assistants de chemin de Files-profile utilisent le contrat de chemin
C-string UTF-8 terminé par NUL de l'ABI v1. La documentation des enveloppes
NE DOIT PAS (MUST NOT) présenter ces assistants comme une couverture complète
des chemins à caractères larges Windows ; les futures fonctions de chemin à
caractères larges DEVRAIENT (SHOULD) être de nouveaux symboles C ABI additifs
en vertu de la politique de compatibilité.

## Pont d'archives compatible Tar

Le pont tar Rust rend GTS utilisable comme une surface d'archive signée, en ajout uniquement (append-only) et dédupliquée pour les utilisateurs qui comprennent déjà tar. `gts from-tar` importe des flux tar dans des archives GTS files-profile-v2, `gts to-tar` exporte ces archives vers tar, et `gts tar -c/-x/-t/-d` fournit la forme de commande familière create/extract/list/diff. Le pont gère les flux `.tar`, `.tar.gz` et `.tar.zst` simples, préservant les métadonnées équivalentes à tar et les enregistrements PAX inconnus là où le profil (profil) peut les représenter.

Pour les grandes archives, les chemins d'importation/création Rust évitent que la mise à l'échelle de la mémoire résidente ne dépende des octets de charge utile des fichiers réguliers sur les chemins de création GTS directs : `gts from-tar` décode l'entrée tar en tant que flux, met en file d'attente (spools) les corps de fichiers réguliers tout en collectant des métadonnées triées, et émet des trames (frames) de blobs à partir de segments (chunks) limités ; gts tar -cf out.gts ... hache et écrit les charges utiles des fichiers sources dans des segments (chunks) limités. Le chemin `to-tar` replié (folded) exporte toujours à partir de la représentation en mémoire `Graph`, et la sortie `.tar.zst` utilise toujours le chemin actuel du moteur (backend) zstd qui matérialise la projection encodée. Ce sont des limites d'implémentation, pas des exigences de format.

L'artéfact canonique devrait être le fichier `.gts` lorsque la vérification est importante : les identifiants de trames (frames), les signatures facultatives, les révisions en ajout uniquement (append-only), les suppressions et les blobs adressés par le contenu restent visibles pour les lecteurs (readers) GTS. Les sorties conventionnelles `.tar`, `.tar.gz` et `.tar.zst` sont des projections de compatibilité utiles pour les chaînes d'outils (toolchains) qui ne parlent pas encore GTS, mais elles devraient être traitées comme des exportations dérivées lorsque la chaîne GTS signée est l'enregistrement de preuve.

Les registres d'artéfacts et les magasins d'objets peuvent transporter l'archive GTS directement en utilisant `application/vnd.blackcat.gts+cbor-seq`. Les éditeurs d'OCI ou d'actifs de version (release-asset) peuvent expédier l'artéfact `.gts` aux côtés des projections tar générées : le registre obtient une archive unique adressée par le contenu pour les consommateurs avertis de GTS, tandis que les consommateurs tar existants conservent un chemin de téléchargement familier. La même séparation fonctionne pour les bundles OKF : le répertoire OKF modifiable reste la surface de création humaine, `gts from-okf` crée le paquet de profil (profil) sémantique `okf`, et le pont tar files-profile-v2 peut empaqueter les octets du répertoire sous la forme d'un artéfact de distribution vérifiable de type tarball lorsque les consommateurs ont besoin d'outils d'archivage ordinaires.

## OKF : Catalogues de connaissances et lots BigQuery

L'interopérabilité OKF dispose de deux passerelles utiles :

- La passerelle hermétique et validée est `vectors/okf/bigquery-join/`. Elle modélise
  les jeux de données BigQuery, les tables, les jointures de tables, le frontmatter d'extension, les liens Markdown,
  et les fichiers de navigation `index.md` sans dépendre des identifiants Google ou
  de la dérive des échantillons en amont.
- La passerelle de l'écosystème en direct est
  <https://github.com/GoogleCloudPlatform/knowledge-catalog>. Ses échantillons `okf/bundles/`
  sont produits par la preuve de concept d'enrichissement OKF du Catalogue de connaissances,
  et son visualiseur consomme la même surface de répertoire Markdown + YAML-frontmatter.

La séquence de commandes Rust pour l'une ou l'autre passerelle est :

```bash
cargo run --features okf --bin gts -- from-okf okf-bundle/ -o bundle.gts
cargo run --bin gts -- verify bundle.gts
cargo run --features okf --bin gts -- to-okf bundle.gts --directory restored-okf/
```

`from-okf` importe les documents de concept avec le frontmatter YAML et traite
les fichiers `index.md` sans frontmatter comme des pages de navigation. Ces pages ne sont pas
des concepts dans le profil GTS, elles ne sont donc pas émises par `to-okf` ; les consommateurs qui
ont besoin de pages de navigation statiques peuvent les régénérer à partir de l'ensemble de concepts exportés.

Le pont positionne OKF comme une interface de création humaine pour les connaissances GMEOW :
les personnes et les agents modifient le Markdown, tandis que GTS fournit l'emballage en ajout uniquement,
les corps adressés par le contenu, les signatures, les suppressions et les projections de graphes pour
l'audit et l'utilisation par les machines.

## Python : rdflib et trames de données (data frames)

Le paquet Python possède le pont d'écosystème le plus riche car il comporte déjà des extensions facultatives pour les cibles RDF et les bases de données :

```bash
pip install 'gmeow-gts[rdf,db]'
```

Aller-retour de l'ensemble de données (dataset) RDF 1.1 :

```python
import gts
from rdflib import Dataset, Literal, URIRef
from rdflib.namespace import RDFS

ds = Dataset()
graph = ds.graph(URIRef("https://example.org/graph"))
graph.add((
    URIRef("https://example.org/Cat"),
    RDFS.label,
    Literal("Cat", lang="en"),
))

data = gts.from_rdflib(ds)
folded = gts.read(data)
assert sorted(gts.to_nquads(folded).splitlines()) == sorted(
    ds.serialize(format="nquads").splitlines()
)

back = gts.to_rdflib(folded)
```

Limitation de RDF 1.2 :

- L'analyseur d'ensemble de données RDF 1.1 stable de rdflib ne représente pas fidèlement les termes de triplets cités (quoted-triples) de GTS ou la syntaxe `rdf:reifies <<( ... )>>`.
- `gts.to_rdflib(graph)` lève `RDF12UnsupportedError` lorsque la projection N-Quads contient des triplets cités.
- `gts.to_rdflib(graph, allow_rdf12_lossy=True)` abandonne les lignes N-Quads contenant la syntaxe de triplets cités et analyse le graphe compatible RDF 1.1 restant.

Le transfert relationnel/trames de données est exposé par Python et Rust :

```bash
gts to-sqlite package.gts package.sqlite
gts to-duckdb package.gts package.duckdb
gts to-parquet package.gts out-parquet/
```

Les exportations Python DuckDB et Parquet nécessitent `pip install 'gmeow-gts[db]'`. Rust utilise `sqlite3` pour SQLite par défaut. Les exportations Rust DuckDB/Parquet sont disponibles lors d'une construction avec `--features duckdb` ; elles n'ajoutent aucune dépendance de crate Rust et font appel à `duckdb` sur `PATH`.

Attente de performance : ces exportations utilisent le modèle replié (folded) par identifiant entier. `terms`, `quads`, `reifiers`, `annotations` et `blobs` sont chargés en masse sans résolution d'IRI pendant l'exportation ; les consommateurs effectuent des jointures via la table `terms`. Le chemin Rust écrit les lignes de manière incrémentielle dans `sqlite3`/`duckdb`, il ne conserve donc pas toutes les lignes SQL ou un script de chargement complet à la fois. La table `blobs` préserve toujours les octets de charge utile (payload), de sorte que les charges utiles blob en ligne transformées sont décodées de manière transitoire lorsque leur ligne est émise. SQLite est adéquat pour une petite inspection locale. DuckDB et Parquet sont les chemins privilégiés pour les analyses de type Pandas, Polars, DuckDB SQL et Arrow car ils conservent l'encodage par dictionnaire et laissent le moteur cible choisir l'ordre de projection/filtrage.

## Rust : crates RDF

L'interopérabilité Rust actuelle maintient la crate par défaut explicite et à faible dépendance :

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let nquads = gmeow_gts::nquads::to_nquads(&graph);
```

Les applications peuvent fournir `nquads` à Sophia, Rio, Oxigraph ou d'autres crates RDF.
Il s'agit du pont stable pour la v1 car la crate principale NE DEVRAIT PAS (SHOULD NOT) imposer une base de données de graphes ou une boîte à outils RDF à chaque utilisateur de transport.

Le chemin inverse en graphe pur est également explicite :

```rust
let bytes = gmeow_gts::from_nquads::from_nquads(nquads.as_str())?;
```

Pour l'interopérabilité native du modèle de données Rust, activez la fonctionnalité optionnelle `rdf` :

```toml
gmeow-gts = { version = "0.9.10", default-features = false, features = ["rdf"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let dataset = gmeow_gts::rdf::to_rdf_dataset(&graph)?;
let bytes = gmeow_gts::rdf::from_rdf_dataset(&dataset)?;
```

La fonctionnalité `rdf` utilise les types de jeux de données RDF, quad, terme, nom de graphe, littéral et triple cité natifs de GTS. Elle ne dépend délibérément pas de la crate `oxrdf` ou d'un magasin RDF externe, de sorte que `--features rdf` reste adapté aux constructions `wasm32-unknown-unknown`.

Pour l'interopérabilité native du magasin RDF en mémoire, activez le magasin natif optionnel :

```toml
gmeow-gts = { version = "0.9.10", default-features = false, features = ["native-store"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let package = gmeow_gts::native_store::graph_to_store_with_sidecar(graph)?;
let writer = gmeow_gts::writer::Writer::from_store(&package.store, "dist")?;
```

La fonctionnalité `native-store` ne dépend que de `rdf`. La projection du magasin est en RDF pur ; l'état exclusif à GTS tel que les blobs, les suppressions, les signatures, les diagnostics, les têtes de segment et les métadonnées de disposition diffusable en continu est renvoyé dans un side-car. L'adaptateur parcourt les quads natifs et ne matérialise pas le texte N-Quads dans le chemin critique.

Pour Sophia, Oxigraph, Rio ou d'autres crates RDF externes, maintenez la dépendance à la frontière de l'application et échangez du texte N-Quads avec GTS. La crate Rust principale ne publie pas d'adaptateur Sophia intégré, car la pile N-Quads de Sophia entraîne la génération d'UUID dans le graphe de dépendances de toutes les fonctionnalités. Les fonctionnalités natives `rdf`, `native-store` et `rdf-codecs` couvrent les chemins d'interopérabilité structurés et textuels intégrés tout en préservant les constructions `wasm32-unknown-unknown`.
L'IC traite également le wasm toutes fonctionnalités (all-features) comme un contrat de bibliothèque Rust permanent :
`scripts/check_rust_wasm_dependency_audit.py` vérifie l'arbre de dépendances normal/de construction `wasm32-unknown-unknown --all-features` et échoue si Oxigraph/OxRDF/OxTTL/OxRDFXML, les crates Sophia, `uuid` ou `getrandom` 0.3 sont présents.

L'exportation stricte est la valeur par défaut. Les réificateurs GTS se projettent vers des termes de triple RDF 1.2 en position d'objet. Si un graphe GTS utilise des triples cités dans des positions que la surface du jeu de données natif ne représente intentionnellement pas, comme la position du sujet ou du nom de graphe, `to_rdf_dataset` lève `RdfAdapterError`. Le chemin explicite `to_rdf_dataset_lossy` abandonne uniquement ces lignes non représentables et est couvert par des tests protégés par des fonctionnalités.

Pour la parité d'application, la crate Rust inclut un exemple de mémoire ancrée exécutable :

```bash
cargo run --example agent_memory
```

`gmeow_gts::examples::agent_memory::Memory` ajoute des revendications, révise ou supprime des revendications, enregistre la provenance des appels d'outils, rappelle des revendications avec un chevauchement de jetons déterministe et produit des paquets acceptés par `gts verify`. Ceci est un exemple d'application s'appuyant sur GTS, et non un prérequis pour les lecteurs principaux.

Le transfert de trame de données Rust utilise les mêmes tableaux repliés que les exportations Python :

```bash
gts to-sqlite package.gts package.sqlite
gts to-duckdb package.gts package.duckdb
gts to-parquet package.gts out-parquet/
```

Le binaire Rust conserve ceux-ci en tant qu'intégrations d'outils d'exécution plutôt qu'en tant que dépendances de crate par défaut :
`to-sqlite` invoque `sqlite3` dans la construction par défaut, tandis que `to-duckdb` et `to-parquet` sont activés par la fonctionnalité Cargo sans dépendance `duckdb` et invoquent le binaire externe `duckdb`. Le chargeur Rust diffuse le SQL des lignes vers ces outils et orchestre le remplacement de la sortie ; il ne construit pas l'ensemble complet de lignes ou le script SQL en mémoire.
Report différé suivi : les adaptateurs RDF Rust natifs supplémentaires ne DEVRAIENT (SHOULD) être ajoutés qu'en tant que fonctionnalités facultatives. Rio reste différé jusqu'à ce qu'une crate compatible Rio maintenue ou qu'un chemin de remplacement soit sélectionné. Tout futur adaptateur DOIT (MUST) inclure des tests d'aller-retour pour les IRI, les nœuds blancs, les littéraux de langue, les types de données, les graphes nommés et les limitations du réificateur RDF 1.2, NE DOIT PAS (MUST NOT) ajouter de dépendance par défaut à une base de données intégrée, et DOIT (MUST) documenter le comportement de triplet cité lorsque la crate cible ne peut pas le préserver.

## TypeScript : Navigateur et récupération par plage (Range Fetch)

Le paquet TypeScript expose un point d'entrée spécifique au navigateur pour les Web Streams :

```typescript
import { foldStream, foldStreamToSink, readStream, toNQuads } from "@blackcatinformatics/gmeow-gts/browser";

const response = await fetch("/artifacts/example.gts");
const result = await foldStream(response.body!, {
  onEvent(event) {
    if (event.kind === "quad") renderQuad(event.quad);
    if (event.kind === "blob") renderBlob(event.digest, event.size);
  },
});

console.log(toNQuads(result.graph));

await foldStreamToSink(response.body!, {
  onEvent(event) {
    if (event.kind === "quad") projectRow(event.segmentIndex, event.quad);
  },
});
```

Le chemin pour navigateur peut également utiliser WebCrypto de la plateforme pour une vérification et un déchiffrement COSE pratiques :

```typescript
const graph = await readStream(response.body!, {
  keys: {
    verificationKey: (kid) => lookupEd25519PublicKey(kid),
    contentKey: (kid) => lookupAes256GcmContentKey(kid),
  },
});
```

L'exportation pour navigateur émet les événements term, quad, reifier, annotation, suppression, blob, opaque, signature, diagnostic, segment-head et streamable-layout dans l'ordre des trames (frame). `foldStreamToSink` est la surface `GTS Streaming Reader` non matérialisante du paquet TypeScript ; `foldStream` et `readStream` demeurent des commodités retournant des graphes. L'API Node `Read(bytes, allowSegments)` racine reste un lecteur (reader) matérialisant, et le code pour navigateur ne doit pas dépendre des assistants CLI/système de fichiers exclusifs à Node.

Règle de plage (Range rule) : les appelants peuvent utiliser HTTP `Range` uniquement pour les étendues d'octets qui sont connues à partir d'une trame (frame) d'index ou d'un balayage séquentiel des limites CBOR. Une plage qui coupe un élément CBOR est un ajout déchiré (torn append) et doit être traitée comme un préfixe incomplet.

## Go : services et magasins d'objets

Les appelants Go DEVRAIENT (SHOULD) utiliser `reader.ReadFrom` aux frontières de services :

```go
func handleGTS(w http.ResponseWriter, r *http.Request) {
    graph, err := reader.ReadFrom(r.Context(), r.Body, reader.Options{
        AllowSegments: true,
        MaxBytes:      64 << 20,
    })
    if err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }
    _, _ = io.WriteString(w, nquads.ToNQuads(graph))
}
```

La même API fonctionne pour les lecteurs de SDK de magasins d'objets :

```go
obj, err := client.GetObject(ctx, bucket, key)
if err != nil {
    return nil, err
}
defer obj.Body.Close()

graph, err := reader.ReadFrom(ctx, obj.Body, reader.Options{
    AllowSegments: true,
    ExpectedHead:  expectedHead,
    MaxBytes:      512 << 20,
})
```

`ReadFrom` est intentionnellement un wrapper de lecteur complet borné. Il offre aux services Go une annulation et des limites de ressources idiomatiques lorsque l'appelant souhaite un `*model.Graph` matérialisé. Le graphe retourné contient toujours les diagnostics du lecteur au lieu de transformer les diagnostics de format en erreurs Go.

Pour les replis diffusables en continu, les appelants peuvent envoyer des événements de repli locaux au segment vers un puits sans construire le graphe d'union final :

<!-- markdownlint-disable MD010 -->
```go
var sink reader.StreamingSink = reader.StreamingSinkFunc(func(event reader.StreamingEvent) error {
	if event.Kind == reader.StreamingEventQuad {
		// project or forward event.Quad here
	}
	return nil
})

result, err := reader.ReadToSink(ctx, obj.Body, reader.Options{
	AllowSegments: true,
	ExpectedHead:  expectedHead,
	MaxBytes:      512 << 20,
}, sink)
```
<!-- markdownlint-enable MD010 -->

`result.Diagnostics`, `result.SegmentHeads` et `result.SegmentStreamable` correspondent au lecteur complet pour les mêmes entrées et options.

## Réplication et frontières de service

Pour les services actuels :

- Utilisez `gts heads` / `gts segments` dans n'importe quel moteur pour inventorier les têtes de segment et les plages d'octets.
- Utilisez `gts ls` ou `Graph.Blobs`/`BlobMeta` repliés pour inventorier les objets en ligne.
- Utilisez les règles de plage ci-dessus lors du service de plages d'octets à partir de HTTP ou de magasins d'objets.
- `gts missing` et `gts resume` fournissent la surface de reprise de plage d'octets stable dans chaque moteur.
  Les protocoles de service à service de niveau supérieur restent du code d'application construit sur les formes JSON et les règles de frontière de
  [GTS-ADVANCED-PRIMITIVES.md](./GTS-ADVANCED-PRIMITIVES.md).

## Garde du contrat

`scripts/check_ecosystem_contract.py` vérifie que ce document conserve la
matrice d'état, les sections par écosystème, le langage de report et les liens vers la documentation publique.
Il s'agit d'un garde-fou contre la dérive pour les promesses d'intégration, et non d'un substitut aux tests du moteur.
