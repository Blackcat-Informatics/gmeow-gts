<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-API-CLI-PARITY.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Contrat de parité API et CLI GTS

> Traduction informative de [`docs/GTS-API-CLI-PARITY.md`](../../../docs/GTS-API-CLI-PARITY.md). Le document anglais demeure la source normative pour les règles de compatibilité, les déclarations de conformité, les matrices de parité, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

Ce document définit la surface multi-langage que Rust, Python, Go, TypeScript, Smalltalk/Pharo et Kotlin/JVM maintiennent compatible alors que les moteurs continuent d'exposer des idiomes natifs. Le format de transfert demeure normatif dans [`GTS-SPEC.md`](./GTS-SPEC.md), et les règles de corpus/niveau demeurent normatives dans [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md). Ce contrat possède la forme de l'API publique et la matrice de parité CLI afin que les écarts de fonctionnalités soient explicites plutôt que déduits de la documentation spécifique à chaque paquet.

L'ABI C reposant sur Rust et les wrappers compatibles C dérivés constituent une couche d'interopérabilité distincte. Ils consomment `libgts` à `rust/capi/include/gts.h`, exposent des API de bibliothèque natives de l'écosystème et n'ajoutent pas de colonnes aux tableaux de parité API/CLI du moteur complet ci-dessous.

## Structure d'API neutre par rapport au langage

La taille stable est sémantique, non syntaxique. Chaque moteur PEUT (MAY) utiliser des noms et des conteneurs natifs, mais les opérations et les champs repliés (folded fields) suivants constituent la cible de compatibilité.
<!-- api-parity-shape:start -->
| operation | contract | current native surface |
|---|---|---|
| `read(input, options)` | Parse a byte buffer or path as a CBOR Sequence, verify the id/prev chain, fold every recoverable frame, and return a graph/result with diagnostics instead of panicking on malformed input. | Python `gts.read(data, keys=None, expected_head=None, allow_segments=True)`; Rust `reader::read(&bytes, allow_segments, expected_head)` or `reader::read_with_options` with `ReadOptions::with_content_key`; Go `reader.Read(data, allowSegments, expectedHead)`; TypeScript `Read(bytes, allowSegments, expectedHead?)`; Smalltalk `GtsReader read:allowSegments:`; Kotlin `read(data, allowSegments)`. |
| `verify(input, options)` | Apply strict transport checks over the same fold: chain/hash diagnostics, expected-head freshness when provided, streamable-layout checks when requested, and COSE signature status when keys are provided. | CLI `gts verify`; Python `gts.verify.verify_file`; Rust `gmeow_gts::verify::verify_file` plus folded diagnostics and lower-level COSE helpers in every engine. |
| `write(graph/events, options)` | Emit deterministic CBOR for hashed or signed bytes, compute each frame id from its content, and set `prev` to the previous frame id. | Python `Writer`; Rust `writer::Writer`; Go `writer.New`; TypeScript `Writer`; Smalltalk `GtsWriter`; Kotlin `Writer`. |
| `fold(input)` | Return the deterministic GTS value fold: terms, quads, reifiers, annotations, blobs, suppressions, opaque nodes, signatures, segment heads, profiles, and streamable layout state. | Same object returned by `read`. |
| `to_nquads(graph)` | Project the folded RDF dataset to sorted N-Quads text with the same value semantics across engines. | Python `to_nquads`; Rust `nquads::to_nquads`; Go `nquads.ToNQuads`; TypeScript `toNQuads`; Smalltalk `GtsNQuads`; Kotlin `toNQuads`. |
| `from_nquads(input)` | Build a GTS file from N-Quads text using the shared writer semantics. | Python `from_nquads`; Rust `from_nquads::from_nquads`; Go `fromnquads.FromNQuads`; TypeScript `fromNQuads`; Smalltalk `GtsFromNQuads`; Kotlin `fromNQuads`; CLI `gts from-nq` in every engine. |
| `to_ntriples(graph)` / `from_ntriples(input)` | Project a default-graph RDF dataset to N-Triples and rebuild GTS bytes from N-Triples text using the shared RDF 1.2 parser/serializer. | Rust `rdf_codecs::to_ntriples` / `from_ntriples` behind `--features rdf-codecs`; Go `rdfcodecs.ToNTriples` / `FromNTriples`; CLI `gts to-nt` and `gts from-nt` in Rust and Go. |
| `to_rdf_xml(graph)` / `from_rdf_xml(input)` | Project a default-graph RDF dataset to RDF/XML and rebuild GTS bytes from RDF/XML text, including RDF/XML namespace, parseType, collection, reification, annotation, and RDF 1.2 triple-term grammar. | Rust `rdf_codecs::to_rdf_xml` / `from_rdf_xml` behind `--features rdf-codecs`; Go `rdfcodecs.ToRDFXML` / `FromRDFXML`; CLI `gts to-rdfxml` and `gts from-rdfxml` in Rust and Go. |
| `to_trig(graph)` / `from_trig(input)` | Project folded RDF to readable TriG graph blocks and rebuild GTS bytes from the supported TriG surface without changing N-Quads content. | Python `gts.trig.to_trig` / `from_trig`; Rust `trig::to_trig` / `from_trig::from_trig`; Rust `rdf_codecs::to_trig` / `from_trig` with `--features rdf-codecs`; Go `rdfcodecs.ToTriG` / `FromTriG`; CLI `gts to-trig` and `gts from-trig` in Python, Rust, and Go. |
| `to_turtle(graph)` / `from_turtle(input)` | Project a default-graph RDF dataset to Turtle and rebuild GTS bytes from Turtle text using the shared Turtle-family RDF 1.2 parser/serializer. | Rust `rdf_codecs::to_turtle` / `from_turtle` behind `--features rdf-codecs`; Go `rdfcodecs.ToTurtle` / `FromTurtle`; CLI `gts to-turtle` and `gts from-turtle` in Rust and Go. |
| graph iterators/accessors | Expose resolved access to terms, quads, reifier bindings, annotations, suppressions, blobs, opaque nodes, signatures, diagnostics, segment heads, profiles, metadata, and streamable state. | Native fields on `Graph`/`GtsGraph` in all six engines, with helper lookups where idiomatic. |
| blobs | Preserve inline blob bytes by `blake3:<hex>` digest and retain declared blob metadata such as media type. Extraction MUST re-hash bytes before writing them. Implementations MAY keep transformed blob bytes lazy until access. | Python `Graph.blobs`/`blob_meta`; Rust `Graph.blobs` lazy `BlobEntry` plus `blob_entry`/`blob_bytes`/`decoded_blobs`; Go `Graph.Blobs`/`BlobMeta`; TypeScript `Graph.blobs`/`blobMeta`; Smalltalk `GtsGraph blobs`/`blobMeta`; Kotlin `Graph.blobs`/`blobMeta`. |
| opaque nodes | Preserve undecodable or unsupported recoverable frames as graph-visible opaque nodes with a frame id, frame type, reason, and signature status. | `OpaqueNode` in every engine. |
| diagnostics | Preserve stable diagnostic `code` values and optional frame indexes; native detail text may differ. | `Diagnostic.code/detail/frame_index`, `Diagnostic { code, detail, frame_index }`, `Diagnostic{Code, Detail, FrameIndex}`, `Diagnostic.code/detail/frameIndex`. |
| streaming/full-reader options | Carry read mode, segment allowance, expected head, key provider, recursion/decode budgets, and streamable validation as options. Engines MAY stage these as separate helpers while preserving the same observable fold and diagnostics. | Python `keys`, Rust `ReadOptions`/`read_to_sink_with_options`/`read_to_sink_from_reader`, Go `reader.Options`/`reader.ReadToSink`, TypeScript `allowSegments`/`foldStreamToSink`, Smalltalk `allowSegments`, Kotlin `allowSegments`, and CLI flags today; deeper recursion/MMR options are future Full Reader work. |
<!-- api-parity-shape:end -->
## Cibles d'égalité inter-langages

Le corpus de conformité compare les champs observables qui rendent les moteurs substituables. Les nouveaux tests et ajouts à l'API DEVRAIENT (SHOULD) préserver ces cibles :

| cible | règle d'égalité |
|---|---|
| folded graph | Les termes, quads, réificateurs, annotations, suppressions, déclarations de profil, métadonnées, l'état diffusable en continu et la projection N-Quads correspondent au JSON attendu. |
| diagnostics | L'ordre des codes de diagnostic correspond. Le texte de détail natif et les enveloppes d'exception/avertissement natives ne sont pas figés. |
| head id | Les ids de tête de segment correspondent en tant qu'hexadécimal minuscule. L'id de tête du dernier segment d'un fichier à segment unique est la tête du fichier pour les vérifications de fraîcheur. |
| opaque reasons | Les chaînes de raison de nœud opaque correspondent après tri, incluant `unknown-codec`, `missing-key`, `damaged` et `unknown-frame-type`. |
| signature status | Le statut de signature par trame utilise `valid`, `invalid` ou `unverified` avec des ids de clé correspondants lorsqu'ils sont présents. |
| blob digests | Les clés de condensé `blake3:<hex>`, les types de média déclarés et les longueurs d'octets décodées correspondent ; l'extraction recalcule le hachage des octets avant l'écriture. |

## Diagnostics et mappage des erreurs natives

Les diagnostics du lecteur sont des données, et non un flux de contrôle lancé, pour les lectures permissives. Les commandes de vérification stricte et de publication PEUVENT (MAY) convertir tout diagnostic d'erreur ou fatal en un code de sortie de processus non nul ou un retour d'erreur native.

| concept | Python | Rust | Go | TypeScript | Smalltalk | Kotlin |
|---|---|---|---|---|---|---|
| enregistrement de diagnostic | `gts.Diagnostic` dataclass | `model::Diagnostic` struct | `model.Diagnostic` struct | `Diagnostic` interface | `GtsDiagnostic` object | `Diagnostic` data class |
| champ de code | `code: str` | `code: String` | `Code string` | `code: string` | `code` | `code: String` |
| champ de détail | `detail: str` | `detail: String` | `Detail string` | `detail: string` | `detail` | `detail: String` |
| index de trame | `frame_index: int \| None` | `frame_index: Option<usize>` | `FrameIndex *int` | `frameIndex?: number` | `frameIndex` | `frameIndex: Int?` |
| résultat de lecture permissive | `Graph` avec `diagnostics` | `Graph` avec `diagnostics` | `*model.Graph` avec `Diagnostics` | `Graph` avec `diagnostics` | `GtsGraph` avec `diagnostics` | `Graph` avec `diagnostics` |
| échec de l'interface CLI stricte | exit `1` pour diagnostics/refus | exit `1` pour diagnostics/refus | exit `1` pour diagnostics/refus | exit `1` pour diagnostics/refus | exit `1` pour diagnostics/refus | exit `1` pour diagnostics/refus |
| échec d'utilisation ou d'E/S | exit `2` | exit `2` | exit `2` | exit `2` | exit `2` | exit `2` |

Le registre canonique des codes de diagnostic se trouve dans [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md#6-diagnostics-registry).

## Matrice de parité CLI

`yes` signifie que le verbe est implémenté par le binaire `gts` de ce moteur. `no` signifie que l'absence est une lacune publique intentionnelle jusqu'à ce qu'un ticket de parité soit résolu. La matrice est vérifiée par [`scripts/check_cli_parity.py`](../scripts/check_cli_parity.py), qui lit ce tableau et les surfaces de répartition réelles.

<!-- cli-parity-matrix:start -->
| verb | Python | Rust | Go | TypeScript | Smalltalk | Kotlin | status |
|---|---|---|---|---|---|---|---|
| `info` | yes | yes | yes | yes | yes | yes | common |
| `fold` | yes | yes | yes | yes | yes | yes | common |
| `verify` | yes | yes | yes | yes | yes | yes | common |
| `extract-key` | yes | yes | yes | yes | yes | yes | common |
| `ls` | yes | yes | yes | yes | yes | yes | common |
| `extract` | yes | yes | yes | yes | yes | yes | common |
| `cat` | yes | yes | yes | yes | yes | yes | common |
| `compact` | yes | yes | yes | yes | yes | yes | common |
| `pack` | yes | yes | yes | yes | yes | yes | common |
| `unpack` | yes | yes | yes | yes | yes | yes | common |
| `diff` | yes | yes | yes | yes | yes | yes | common |
| `from-nq` | yes | yes | yes | yes | yes | yes | common |
| `to-trig` | yes | yes | yes | no | no | no | Python/Rust/Go transform extension |
| `from-trig` | yes | yes | yes | no | no | no | Python/Rust/Go transform extension |
| `to-nt` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `from-nt` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `to-rdfxml` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `from-rdfxml` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `to-turtle` | no | yes | yes | no | no | no | Rust/Go Turtle-family transform extension |
| `from-turtle` | no | yes | yes | no | no | no | Rust/Go Turtle-family transform extension |
| `to-yaml-ld` | no | yes | no | no | no | no | Rust transform extension |
| `from-yaml-ld` | no | yes | no | no | no | no | Rust transform extension |
| `to-okf` | no | yes | no | no | no | no | Rust OKF profile extension |
| `from-okf` | no | yes | no | no | no | no | Rust OKF profile extension |
| `to-tar` | no | yes | no | no | no | no | Rust tar bridge extension |
| `from-tar` | no | yes | no | no | no | no | Rust tar bridge extension |
| `tar` | no | yes | no | no | no | no | Rust tar-compatible extension |
| `to-sqlite` | yes | yes | no | no | no | no | Python/Rust extension |
| `to-duckdb` | yes | yes | no | no | no | no | Python/Rust extension |
| `to-parquet` | yes | yes | no | no | no | no | Python/Rust extension |
| `prove` | no | yes | no | no | no | no | Rust proof creation extension |
| `dump` | no | yes | no | no | no | no | Rust inspection export extension |
| `verify-proof` | yes | yes | yes | yes | yes | yes | common |
| `heads` | yes | yes | yes | yes | yes | yes | common |
| `segments` | yes | yes | yes | yes | yes | yes | common |
| `missing` | yes | yes | yes | yes | yes | yes | common |
| `resume` | yes | yes | yes | yes | yes | yes | common |
<!-- cli-parity-matrix:end -->
### Lacunes intentionnelles

- Rust ``to-sqlite`` nécessite ``sqlite3`` sur ``PATH`` ; Rust ``to-duckdb`` et ``to-parquet`` nécessitent la fonctionnalité Cargo ``duckdb`` optionnelle sans dépendance plus ``duckdb`` sur ``PATH``. Les exports Python DuckDB et Parquet nécessitent l'extra Python ``[db]``. Rust diffuse les lignes SQL vers l'outil d'exécution au lieu de conserver toutes les lignes relationnelles ou un script SQL complet en mémoire ; le schéma stable ``blobs.bytes`` nécessite toujours le décodage de blobs transitoires pendant que chaque ligne de blob est émise.
- Go, TypeScript, Smalltalk et Kotlin n'exposent pas encore d'exports relationnels.
- ``to-trig`` et ``from-trig`` sont des extensions de transformation Python/Rust/Go. Elles préservent le même contenu RDF replié (fold) que la projection N-Quads tout en utilisant des blocs de graphes TriG lisibles ; la parité pour TypeScript, Smalltalk et Kotlin peut arriver plus tard par rapport aux mêmes attentes de cycle complet (round-trip).
- ``to-nt`` et ``from-nt`` sont des extensions de codec texte RDF Rust/Go. ``to-nt`` n'accepte que les projections RDF de graphe par défaut ; les jeux de données à graphes nommés DEVRAIENT (SHOULD) utiliser ``to-trig``. La parité pour Python, TypeScript, Smalltalk et Kotlin peut arriver plus tard par rapport aux mêmes attentes d'analyseur et de cycle complet (round-trip).
- ``to-rdfxml`` et ``from-rdfxml`` sont des extensions de codec texte RDF Rust/Go. Elles couvrent l'analyse et la sérialisation RDF/XML via le contrat d'événement, incluant les espaces de noms, ``rdf:parseType``, les collections, la réification, les annotations et les surfaces de termes de triplets RDF 1.2. ``to-rdfxml`` n'accepte que les projections RDF de graphe par défaut ; les jeux de données à graphes nommés DEVRAIENT (SHOULD) utiliser ``to-trig``. La parité pour Python, TypeScript, Smalltalk et Kotlin peut arriver plus tard par rapport aux mêmes attentes de la suite W3C RDF/XML.
- ``to-turtle`` et ``from-turtle`` sont des extensions de transformation de la famille Turtle pour Rust/Go. Elles utilisent la même pile d'analyseur/sérialiseur RDF 1.2 que le chemin TriG complet. ``to-turtle`` n'accepte que les projections RDF de graphe par défaut ; les jeux de données à graphes nommés DEVRAIENT (SHOULD) utiliser ``to-trig``. La parité pour Python, TypeScript, Smalltalk et Kotlin peut arriver plus tard par rapport aux mêmes attentes d'analyseur et de cycle complet (round-trip).
- ``to-yaml-ld`` et ``from-yaml-ld`` sont des verbes d'extension exclusifs à Rust derrière ``--features yaml-ld``. Ce sont des couches d'adaptation (shims) de transformation uniquement sur des tables de graphes repliées (fold), et non un changement de format de transmission (wire-format) ou de catalogue canonique ; la parité pour Python, Go, TypeScript, Smalltalk et Kotlin peut arriver plus tard avec l'ajout d'un oracle de corpus partagé si nécessaire.
- ``to-okf`` et ``from-okf`` sont des verbes de profil (profile) OKF exclusifs à Rust derrière ``--features okf``. Ils mappent un lot (bundle) Markdown OKF vers le profil (profile) GTS ``okf`` avec le schéma de manifeste ``gts-okf-v1``, des blobs de corps Markdown adressés par le contenu, des arêtes de liens interrogeables, une tolérance de navigation ``index.md`` et ``_unmapped.nq`` pour le RDF hors profil. Le corpus de conformité OKF commis, incluant ``vectors/okf/bigquery-join/``, est le seuil de parité requis pour toute implémentation future en Python, Go, TypeScript, Smalltalk ou Kotlin. Ces moteurs DOIVENT (MUST) rester ``no`` ici jusqu'à ce qu'ils puissent importer/exporter le contrat de répertoire ``gts-okf-v1`` et préserver les attentes N-Quads repliées (fold).
- ``to-tar``, ``from-tar`` et ``tar`` sont des verbes de pont files-profile-v2 exclusifs à Rust derrière ``--features tar``. Ils mappent des flux tar vers des fichiers GTS et inversement tout en préservant les métadonnées files-profile, les enregistrements de liens/fichiers spéciaux optionnels, l'emballage gzip/zstd, les enregistrements PAX inconnus et une surface de commande ``-c/-x/-t/-d`` compatible avec tar. La parité pour Python, Go, TypeScript, Smalltalk et Kotlin DEVRAIT (SHOULD) arriver plus tard par rapport à la même politique de sécurité et au même comportement de cycle complet (round-trip). Le seuil de parité requis est le corpus de conformité commis ``vectors/tar/`` plus le comportement d'import/export files-profile-v2 ; ces moteurs DOIVENT (MUST) rester ``no`` ici jusqu'à ce qu'ils puissent préserver les mêmes métadonnées de manifeste, la politique de refus et les attentes de cycle complet (round-trip) tar.
- ``dump`` est un export d'inspection exclusif à Rust qui écrit une arborescence de répertoires versionnée avec des N-Quads repliés (fold), des tables JSONL, des vues de trames (frame) dépliées, des index de blobs et des charges utiles files-profile. Ce n'est pas un changement de format de transmission (wire-format) ; la parité pour Python, Go, TypeScript, Smalltalk et Kotlin peut implémenter le même contrat de répertoire ``gts-dump-v1`` plus tard.
- Tous les moteurs implémentent ``verify-proof`` pour le JSON de preuve MMR détaché en utilisant les préimages stables et les montages (fixtures) positifs/négatifs dans ``vectors/proofs/``. Rust implémente en plus ``prove`` à partir de fichiers qui portent une racine ``index.mmr`` vérifiée. Python, Go, TypeScript, Smalltalk et Kotlin ne DEVRAIENT PAS (SHOULD NOT) exposer ``prove`` avant de pouvoir créer des preuves basées sur des fichiers par rapport à la même discipline de montage (fixture).
- Tous les moteurs implémentent les verbes de réplication avec les mêmes schémas JSON et règles de limite de reprise : ``gts-replication-heads-v1``, ``gts-replication-segments-v1`` et ``gts-replication-missing-v1``.
- Les futurs verbes de récursion GTS imbriqués (nested) et de politique de chiffrement ne font pas encore partie de la surface CLI stable. Ils DEVRAIENT (SHOULD) être ajoutés à cette matrice avant que les documents spécifiques aux packages ne les revendiquent.
- Les verbes différés avancés restants, s'il y en a, sont suivis dans [``GTS-ADVANCED-PRIMITIVES.md``](./GTS-ADVANCED-PRIMITIVES.md) et gardés par ``scripts/check_advanced_contract.py``.

## Garde-dérive

Exécutez les vérifications de parité localement avec :

```bash
python scripts/check_api_parity.py
python scripts/check_cli_parity.py
```

La tâche de peluchage (lint) de l'IC exécute les mêmes commandes. La vérification d'API lit [`api-parity.json`](./api-parity.json), ce document, la matrice de fonctionnalités des moteurs du README et des preuves de fumée de bas niveau au niveau de la source pour les six moteurs complets. Les moteurs complets sont Rust, Python, Go, TypeScript, Smalltalk/Pharo et Kotlin/JVM. Les enveloppes (wrappers) dérivées de l'ABI C restent déclarées séparément et NE DOIVENT PAS (MUST NOT) devenir des colonnes de parité de moteur complet.

La vérification d'API échoue quand :

- le tableau de forme de l'API rendu change sans la mise à jour de déclaration correspondante ;
- la matrice de fonctionnalités du README change sans la mise à jour de déclaration correspondante ;
- une revendication de prise en charge manque de preuve source ou si l'exportation/le module/le fichier public déclaré disparaît ;
- une revendication différée n'est pas explicitement enregistrée comme telle ;
- une surface d'enveloppe est ajoutée à la déclaration du moteur complet.

La vérification de CLI échoue quand :

- un moteur implémente un verbe de CLI non représenté dans la matrice ;
- la matrice marque un verbe `yes` pour un moteur dont la surface de répartition en est dépourvue ;
- la matrice marque un verbe `no` pour un moteur qui l'implémente désormais ;
- les blocs de commandes communs et d'extension Python du README dérivent de la matrice.

Lors de l'ajout d'une revendication de parité d'API, mettez à jour l'implémentation/l'exportation du moteur, les preuves sources dans `docs/api-parity.json`, ce tableau de forme d'API ou la matrice de fonctionnalités du README, et les tests au niveau du paquet dans le même changement. Lors du report de la parité pour un moteur, maintenez la cellule du README à `no` et ajoutez une raison de report dans la déclaration au lieu de vous fier à l'omission.

Lors de l'ajout ou de la suppression d'un verbe de CLI, mettez à jour l'implémentation, cette matrice, les blocs de commandes du README et le texte du README spécifique au paquet dans le même changement.

## Surface de l'enveloppe de l'ABI C

La famille d'enveloppes de l'ABI C est intentionnellement plus étroite qu'un moteur complet natif. Les enveloppes délèguent la sémantique de format au moteur Rust et rendent l'ABI stable pratique depuis les écosystèmes compatibles C :

| Surface | Contrat |
|---|---|
| Métadonnées de l'ABI | `gts_abi_version`, `gts_version`, le JSON de métadonnées de construction et le JSON de capacité identifient la surface `libgts` chargée. |
| Lecture/repli | `gts_read_json` renvoie un rapport JSON stable pour l'état de l'archive repliée. |
| Vérifier | `gts_verify_json` renvoie le rapport du vérificateur Rust sous forme de JSON. |
| Formats de texte RDF | `gts_formats_json`, `gts_to_format` et `gts_from_format` exposent la conversion pilotée par le registre pour N-Quads, N-Triples, Turtle, TriG, RDF/XML et le profil JSON-LD-star déterministe. `gts_to_nquads` et `gts_from_nquads` demeurent des assistants de compatibilité. |
| Profil de fichiers | `gts_files_pack`, `gts_files_unpack` et `gts_files_diff_json` exposent des assistants de profil de fichiers. |
| Propriété | Les valeurs `gts_buffer` renvoyées sont copiées dans des chaînes ou des tableaux d'octets natifs de l'écosystème, puis libérées avec `gts_buffer_free`. |
| Erreurs | Les retours `gts_status` qui ne sont pas OK sont copiés depuis les poignées `gts_error` vers des erreurs structurées de l'écosystème, puis libérés avec `gts_error_free`. |

Les enveloppes actuelles sont C++, .NET, PHP, Lua, Swift, Ruby, R et Julia. Chaque README d'enveloppe possède son propre nommage local, son comportement de chargeur, ses notes sur les fils d'exécution et sa commande de test de fumée. Les tests de fumée des enveloppes prouvent l'accessibilité de l'ABI et le comportement de propriété ; ils ne remplacent pas le corpus de conformité complet des six moteurs.

## Contrat de commande du profil Files

``pack``, ``unpack`` et ``diff`` sont des commandes communes aux six moteurs. Leur comportement observable fait partie de la surface de parité :

- ``pack <dir|file>... -o out.gts`` émet un seul segment ``files`` avec les termes/quads du catalogue avant les blobs intégrés, stocke chaque chemin une seule fois et dédoublonne le contenu identique par digest.
- Les chemins d'archive stockés sont des chemins relatifs séparés par ``/``. Chaque moteur refuse les chemins vides, les chemins absolus, les chemins relatifs à un lecteur Windows, ``..``, ``.``, les composants vides et les séparateurs barre oblique inverse avant de lire ou d'écrire les octets du fichier.
- Les liens symboliques ne sont pas archivés. ``pack`` et ``diff`` refusent les entrées de liens symboliques plutôt que de les suivre ; ``unpack`` refuse les chemins qui s'échappent du répertoire de destination, y compris les échappements via des liens symboliques existants sous ce répertoire.
- ``unpack`` recalcule le hachage des octets du blob intégré avant l'écriture. Un ``FileEntry`` non supprimé dont le blob intégré est absent est un refus ; les digests de blobs supprimés sont ignorés par défaut et extraits uniquement avec ``--include-suppressed``.
- ``diff`` compare le manifeste de l'archive à un répertoire par ``files:digest`` et renvoie des lignes ``added:``, ``modified:`` et ``removed:`` triées. Le code de sortie ``0`` signifie qu'il n'y a aucune différence ; le code de sortie ``1`` signifie soit une différence, soit une entrée refusée.

Le garde-fou inter-moteurs en direct est [``scripts/interop.sh``](../scripts/interop.sh) : chaque moteur empaquette la même fixture, chaque moteur replie (folds) et déballe chaque paquet, et chaque moteur compare (diffs) à la fois l'arborescence correspondante et une arborescence modifiée par rapport à chaque paquet.
