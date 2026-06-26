<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-OKF.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Profil GTS OKF

> Traduction informative de [`docs/GTS-OKF.md`](../../../../docs/GTS-OKF.md). Le document anglais demeure la source normative pour les intégrations, les fonctionnalités avancées, les profils optionnels, les données de référence, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md) et reste informative.

La fonctionnalité Rust `okf` mappe un lot Markdown OKF vers un paquet GTS vérifiable
et projette un graphe replié de profil OKF vers un répertoire de lot.

```bash
cargo run --features okf --bin gts -- from-okf okf-bundle/ -o bundle.gts
cargo run --features okf --bin gts -- to-okf bundle.gts --directory restored-okf/
```

Le profil de segment GTS est `okf`. Les métadonnées d'en-tête transportent un manifeste
OKF avec le schéma `gts-okf-v1`, l'IRI de base utilisé pour les sujets créés,
le nombre de documents et les chemins sources.

## Vocabulaire

L'espace de noms du vocabulaire v1 est :

```text
https://blackcatinformatics.ca/projects/gts/okf#
```

L'IRI de base par défaut du document est :

```text
https://blackcatinformatics.ca/projects/gts/okf/doc/
```

## Correspondance

| Construit OKF | Représentation GTS |
|---|---|
| `foo/bar.md` | un nœud sujet RDF |
| IRI de sujet | `resource:` si présent, sinon `base-iri + percent-encoded relative path` |
| chemin de bundle | littéral de chaîne `okf:path` |
| `type:` | littéral de chaîne `okf:type` requis |
| `title:` | littéral de chaîne `okf:title` |
| `description:` | littéral de chaîne `okf:description` |
| `resource:` | IRI `okf:resource` |
| `tags:` | littéraux de chaîne `okf:tag` répétés, réémis triés |
| `timestamp:` | littéral `okf:timestamp` `xsd:dateTime` |
| scalaire d'extension du producteur | littéral `okf:<key>` chaîne, entier, décimal ou booléen |
| objet/tableau/nul d'extension du producteur | littéral JSON `okf:<key>` avec le type de données `okf:json` |
| corps Markdown | littéral `okf:body` portant un condensé `blake3:<hex>` plus un blob en ligne avec le type de média `text/markdown` |
| variante de corps en ligne | littéral de chaîne `okf:body`, accepté par l'exportation uniquement avec `--inline-body` |
| `[text](target.md)` | `okf:links` arête vers le sujet cible, réifiée avec `okf:linkText` et `okf:linkOccurrence` |
| `index.md` sans frontmatter | page de navigation, ignorée par l'importation et régénérée par les consommateurs au besoin |

Le blob de corps fait autorité pour la resérialisation. Les triplets de lien sont des surfaces de requête dérivées du corps ; `to-okf` ne réécrit pas le Markdown à partir de ceux-ci.

## Exportation de répertoire

`to-okf` refuse un répertoire de destination existant. En cas de succès, il écrit :

```text
out/
├── .gts-okf/
│   └── manifest.json
├── concept-a.md
├── nested/
│   └── concept-b.md
└── _unmapped.nq
```

`_unmapped.nq` est présent uniquement lorsque le graphe contient des triplets en dehors du profil OKF, des graphes nommés ou un état de réificateur/annotation non-OKF. Ces triplets sont signalés sur stderr et conservés dans le sidecar au lieu d'être supprimés silencieusement.

## Manifeste

`.gts-okf/manifest.json` utilise le schéma `gts-okf-v1` :

```json
{
  "schema": "gts-okf-v1",
  "base_iri": "https://blackcatinformatics.ca/projects/gts/okf/doc/",
  "doc_count": 2,
  "source_paths": ["concept-a.md", "nested/concept-b.md"],
  "unmapped_triples": 0
}
```

Les métadonnées d'en-tête GTS portent les mêmes nom de schéma, IRI de base, nombre de documents et liste de chemins sources pour une provenance vérifiable à l'intérieur du paquet.

## Interopérabilité Knowledge Catalog

L'importateur Rust accepte la forme OKF v0.1 utilisée par les échantillons de preuve de concept de Knowledge Catalog de Google :

- les documents de concept sont des fichiers Markdown UTF-8 avec un frontmatter YAML ;
- `type:` est la seule clé de frontmatter requise ;
- `title:`, `description:`, `resource:`, `tags:`, `timestamp:`, et les clés d'extension de producteur arbitraires sont préservées dans le profil (profile) de graphe OKF ;
- les liens Markdown ordinaires entre les fichiers de concept deviennent des arêtes `okf:links` interrogeables ;
- les fichiers `index.md` sans frontmatter sont traités comme des pages de navigation, et non comme des concepts.

L'élément fixe du corpus de conformité (conformance corpus) `vectors/okf/bigquery-join/` est la porte d'interopérabilité hermétique de style BigQuery. Il comprend des concepts de table/dictionnaire, du frontmatter d'extension, des liens Markdown relatifs et des pages de navigation `index.md`. La porte amont en direct est le répertoire `okf/bundles/` du dépôt Knowledge Catalog :

```bash
cargo run --features okf --bin gts -- from-okf vectors/okf/bigquery-join -o /tmp/bq.gts
cargo run --bin gts -- verify /tmp/bq.gts
cargo run --features okf --bin gts -- to-okf /tmp/bq.gts --directory /tmp/bq-okf
```

Lors de tests effectués par rapport à une extraction de <https://github.com/GoogleCloudPlatform/knowledge-catalog>, remplacez un lot tel que `okf/bundles/ga4/` par `vectors/okf/bigquery-join`.

## Démonstrateur OKF vérifiable

OKF est « juste un répertoire » au niveau de la couche de création. Le profil GTS ajoute une couche de vérification en ajout uniquement sous ce répertoire :

1. Un humain ou un agent crée un lot OKF en Markdown.
2. `gts from-okf` empaquette le lot dans le profil `okf`, stocke les corps Markdown en tant que blobs adressés par le contenu, et enregistre un manifeste `gts-okf-v1` dans les métadonnées du paquet.
3. Les applications Rust qui nécessitent une garde signée créent des trames avec `Writer::sign_with` ou `Writer::sign_with_openpgp_secret_key` ; `gts verify` vérifie ensuite la chaîne d'identifiants et les observations COSE Sign1.
4. Les révisions ajoutent de nouvelles revendications et trames de suppression au lieu de réécrire l'historique ancien. Le modèle est le même que `gmeow_gts::examples::agent_memory` : ajouter la revendication de remplacement, ajouter la provenance `gmeow:wasDerivedFrom`, et supprimer le terme ou le blob remplacé.
5. `gts to-okf` projette le repli actuel du profil OKF vers le Markdown pour révision ou publication. L'historique supprimé reste dans le paquet GTS pour audit à moins qu'une politique de compactage ultérieure ne le scelle ou ne le réécrive délibérément.

Cela fait d'OKF une surface de création GMEOW : les contributeurs peuvent travailler en Markdown et avec des outils de révision de dépôt ordinaires, tandis que les systèmes en aval peuvent consommer les mêmes connaissances sous forme d'état de graphe GTS signé, suppressible et interrogeable.

## Lois d'aller-retour

Aller-retour OKF direct :

```text
okf-dir -> from-okf -> package.gts -> to-okf -> okf-dir'
```

Le lot restauré est de contenu égal modulo les clés de frontmatter triées, les
étiquettes triées et la canonisation YAML. Les octets du corps Markdown sont
identiques à l'octet près.

Aller-retour GTS inverse :

```text
package.gts -> to-okf -> okf-dir -> from-okf -> package.gts'
```

Pour les graphes de profil OKF, la projection de graphe repliée est égale après
l'aller-retour. Les ID de contenu peuvent différer car l'importateur rédige un
nouveau segment déterministe plutôt que de rejouer les octets sources.

## Rejets

`from-okf` rejette :

- les racines de bundle qui ne sont pas des répertoires ;
- les liens symboliques dans le bundle ;
- les fichiers Markdown sans frontmatter YAML, à l'exception des fichiers de navigation `index.md` ;
- le frontmatter qui n'est pas un mappage ;
- les documents auxquels il manque les `type:` requis ;
- les chemins relatifs non sécurisés ;
- les liens Markdown orphelins lorsque `--strict-links` est passé.

`to-okf` rejette :

- les répertoires de sortie existants ;
- les sujets OKF sans `okf:path` ;
- les documents OKF sans `okf:type` ;
- les blobs de corps manquants ou impossibles à décoder ;
- les littéraux `okf:body` en ligne, à moins que `--inline-body` ne soit passé.

## Relation avec d'autres surfaces de répertoire

`gts dump --directory` écrit une arborescence d'inspection pour les archives GTS arbitraires.
`gts to-okf --directory` écrit une surface de création OKF pour les graphes qui utilisent le
vocabulaire de profil OKF. Il s'agit de contrats de répertoire intentionnellement distincts :
`gts-dump-v1` est destiné à l'examen d'archives, tandis que `gts-okf-v1` est destiné à l'échange
de paquets Markdown.
