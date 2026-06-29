<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-SPEC.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

<a id="gts-graph-transport-substrate-specification"></a>

# GTS — Graph Transport Substrate — Spécification

> Traduction informative de [`docs/GTS-SPEC.md`](../../../GTS-SPEC.md). Le document anglais demeure la source normative pour les règles de protocole, le format filaire, les exigences de conformité, les considérations de sécurité, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

**Version du document :** 0.9-draft &nbsp;·&nbsp; **Version majeure du format filaire :** 1 &nbsp;·&nbsp;
**Date :** 2026-06-18 &nbsp;·&nbsp; **Rédacteur :** Patrick Audley, Blackcat Informatics® Inc. &nbsp;·&nbsp;
**Cette version :** <https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md> &nbsp;·&nbsp;
**DOI :** <https://doi.org/10.67342/6pta6imnmw/v1>

<a id="abstract"></a>

## Résumé

GTS (Graph Transport Substrate) est un conteneur binaire et un format de transport indépendant de l'ontologie pour les jeux de données RDF 1.2 et les charges utiles binaires adressées par contenu. Un fichier GTS est une séquence CBOR d'un ou plusieurs segments en ajout uniquement. Chaque segment est constitué d'un en-tête CBOR déterministe suivi de trames CBOR déterministes liées par des identifiants de contenu BLAKE3. Le jeu de données logique est obtenu par un repli déterministe sur la séquence de segments. GTS prend en charge la lisibilité partielle, les trames opaques chiffrées ou à codec inconnu, la suppression en ajout uniquement, les signatures et le chiffrement facultatifs, ainsi que la conformité inter-langages via un corpus de vecteurs partagé.

<a id="status-of-this-document"></a>

## État de ce document

| Champ | Valeur |

|---|---|

| État | Ébauche de travail |

| Version du document | 0.9-draft |

| Version majeure du format de transmission | 1, encodée dans le champ `"v"` de l'en-tête de segment |

| Date | 2026-06-18 |

| DOI du document | <https://doi.org/10.67342/6pta6imnmw/v1> |

| Stabilité | Des modifications au format de transmission demeurent possibles jusqu'à la v1.0 |

| Contrôle des changements | Blackcat Informatics / [processus de gouvernance GTS](./GTS-GOVERNANCE.md) |

| Conformité | Définie par ce document et le corpus de vecteurs versionné (§19) |

| Versions d'implémentation | Les versions des paquets sont des artefacts de version indépendants |

| Version du corpus | Le corpus est versionné séparément des versions des paquets |

Cette spécification est maintenue dans le dépôt [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts), aux côtés de six moteurs de référence interopérables (Rust, Python, Go, TypeScript, Smalltalk/Pharo, Kotlin/JVM) qui servent de barrière de validation par rapport au corpus de vecteurs partagé. Signalez les errata et proposez des changements à cet endroit. Les changements sémantiques fondamentaux, les ajouts au registre et la promotion de profils de standards optionnels suivent le [processus de gouvernance GTS](./GTS-GOVERNANCE.md).

GTS est indépendant de l'ontologie. GMEOW est un consommateur en aval et un cas d'utilisation de distribution principal pour GTS, mais les lecteurs et rédacteurs GTS ne requièrent pas le vocabulaire, l'outillage ou la sémantique de GMEOW. Des profils spécifiques au domaine, incluant les profils GMEOW et les profils de paquets musicaux, sont superposés au format de base.

<a id="document-history"></a>

## Historique du document

Cette section consigne les modifications apportées à ce document de spécification. Les versions de paquets, les numéros de version de paquets et les notes de version par moteur sont des artefacts distincts et ne sont pas suggérés par la version du document.

**Modifications de la version v0.9-draft (2026-06-18) :**

- Aligne les métadonnées de publication avec l'état actuel de préparation de v1.0-rc1 tout en gardant les versions de paquets indépendantes de la version du document de spécification.

- Clarifie les portées de conformité, les classes de lecteurs (readers)/rédacteurs (writers), les limites de mémoire du lecteur en continu et les diagnostics du lecteur canonique.

- Formalise le repli (fold) de graphe, l'union de valeurs multi-segments, la portée des nœuds vides (blank-node scoping), le triple-term RDF 1.2 et le mappage de `rdf:reifies`, les contraintes de position et le comportement en cas de doublons ou de conflits.

- Ajoute des règles de disposition diffusable en continu (streamable-layout), des pré-images de preuve d'index/MMR facultatives, la vérification de preuve, le comportement des clés d'extension inconnues, les contrats de type de média et de service HTTP, ainsi que des considérations de durabilité et de sécurité.

- Étend les références au corpus de vecteurs et au manifeste afin que les affirmations de conformité nomment les révisions du corpus, les sous-ensembles, les paliers, les modes et les artefacts de manifeste estampillés par version.

- Épingle le substrat RDF 1.2 à la version Snapshot du W3C Candidate Recommendation du 07 avril 2026 et précise quelles sémantiques RDF sont importées par GTS.

**Notes de la version antérieure du document v0.3 :**

- Fichiers multi-segments (composition par ajout `cat`, §3.1) ; IDs de termes à portée de segment (§7.2) ; sémantique de repli (fold) et d'union de valeurs par segment (§7.5) ; suppression inter-segments (§11) ; union de profils et discipline des étiquettes de langue par section (§13) ; exigences des outils de composition (§14.1) ; vecteurs de conformité 15–21 (§19).

- États de disposition et l'affirmation de diffusion en continu (§3.3, §5) ; compactage diffusable en continu avec signatures de trame (frame) détachées (§10.1) ; le vocabulaire `stream` (§13.3) ; le verbe `compact` (§14.1) ; vecteurs de conformité 24–26 (§19).

<a id="table-of-contents"></a>

## Table des matières

- [1. Aperçu et non-objectifs](#1-overview-and-non-goals)

- [2. Terminologie et conformité](#2-terminology-and-conformance)

  - [2.1 Étendues de conformité](#21-conformance-scopes)

  - [2.2 Classes de conformité des lecteurs (readers) et des rédacteurs (writers)](#22-reader-and-writer-conformance-classes)

  - [2.3 Forme de l'API de base du lecteur](#23-baseline-reader-api-shape)

  - [2.4 Diagnostics du lecteur](#24-reader-diagnostics)

- [3. Structure du fichier](#3-file-structure)

  - [3.1 Fichiers multi-segments (composition par ajout `cat`)](#31-multi-segment-files-cat-append-composition)

  - [3.2 Diffusion en continu et amélioration progressive](#32-streaming-and-progressive-enhancement)

  - [3.3 États de disposition : accrétif et diffusable en continu](#33-layout-states-accretive-and-streamable)

- [4. Conventions CBOR](#4-cbor-conventions)

- [5. En-tête](#5-header)

- [6. Trames](#6-frames)

  - [6.1 Résolution de la charge utile](#61-payload-resolution)

  - [6.2 Trame d'index (optionnelle)](#62-index-frame-optional)

- [7. Modèle de données de graphe et repli](#7-graph-data-model-and-fold)

  - [7.1 Termes (trame `terms`)](#71-terms-terms-frame)

  - [7.2 Attribution d'identifiants de termes (normative)](#72-term-id-assignment-normative)

  - [7.3 Triplets cités et réificateurs (trame `reifies`)](#73-quoted-triples-and-reifiers-reifies-frame)

  - [7.4 Quads et annotations](#74-quads-and-annotations)

  - [7.5 Algorithme de repli (normatif)](#75-fold-algorithm-normative)

  - [7.6 Noeuds opaques](#76-opaque-nodes)

  - [7.7 Repli en continu et mémoire bornée](#77-streaming-fold-and-bounded-memory)

  - [7.8 Doublons et conflits (normatif)](#78-duplicates-and-conflicts-normative)

- [8. Catalogue de transformations](#8-transform-catalog)

  - [8.1 Classes](#81-classes)

  - [8.2 Empilement](#82-stacking)

  - [8.3 Modèle de capacité et dégradation gracieuse](#83-capability-model-and-graceful-degradation)

  - [8.4 Ensemble de base obligatoire et durabilité](#84-mandatory-core-set-and-durability)

  - [8.5 Registre de codecs canoniques (v1)](#85-canonical-codec-registry-v1)

- [9. Intégrité et confidentialité](#9-integrity-and-confidentiality)

  - [9.1 Auto-hachage par trame et chaîne d'identifiants de contenu (obligatoire (MANDATORY))](#91-per-frame-self-hash-and-content-id-chain-mandatory)

  - [9.2 Signatures (optionnel, agilité algorithmique)](#92-signatures-optional-algorithm-agile)

  - [9.3 Chiffrement (optionnel)](#93-encryption-optional)

  - [9.4 L'invariant d'opacité (normatif)](#94-the-opacity-invariant-normative)

- [10. Compaction](#10-compaction)

  - [10.1 Compaction diffusable en continu (ordre uniquement)](#101-streamable-compaction-ordering-only)

- [11. Suppression (effacement « additif »)](#11-suppression-additive-deletion)

- [12. Binaire et adressage par le contenu](#12-binary-and-content-addressing)

  - [12.1 GTS imbriqué (composition récursive)](#121-nested-gts-recursive-composition)

- [13. Profils](#13-profiles)

  - [13.1 Discipline des étiquettes de langue (normative au niveau du profil)](#131-language-tag-discipline-profile-level-normative)

  - [13.2 Le profil `files` (standard optionnel)](#132-the-files-profile-optional-standard)

  - [13.3 Le vocabulaire `stream` (standard optionnel)](#133-the-stream-vocabulary-optional-standard)

  - [13.4 Exemple de profil de domaine : `music-package` (informatif)](#134-domain-profile-example-music-package-informative)

- [14. Sortie des transformations](#14-transforms-out)

  - [14.1 Exigences relatives aux outils de composition (normative pour les outils conformes)](#141-composition-tooling-requirements-normative-for-conformant-tools)

  - [14.2 Outils d'archivage (profil `files`)](#142-archive-tooling-files-profile)

- [15. Exemples détaillés](#15-worked-examples)

  - [15.1 Instantané de distribution minimal (`dist`)](#151-minimal-distribution-snapshot-dist)

  - [15.2 Preuve : image + accumulation signée (`evidence`)](#152-evidence-image--signed-accrual-evidence)

  - [15.3 Notaire : trame partiellement opaque (`opaque`)](#153-notary-partially-opaque-frame-opaque)

  - [15.4 Dégradation gracieuse (`image`, négociation de contenu)](#154-graceful-degradation-image-content-negotiation)

  - [15.5 Matryoshka : un GTS entier signé scellé à l'intérieur d'une trame (`bundle` / `opaque`)](#155-matryoshka-a-whole-signed-gts-sealed-inside-a-frame-bundle--opaque)

- [16. Type de média et contrat de service HTTP](#16-media-type-and-http-serving-contract)

  - [16.1 Type de média et extension de fichier (normatif)](#161-media-type-and-file-extension-normative)

  - [16.2 Algorithme d'identification de fichier (normatif)](#162-file-identification-algorithm-normative)

  - [16.3 Sémantique de service HTTP (normative)](#163-http-serving-semantics-normative)

  - [16.4 Mise en cache sensible à l'immuabilité (normative)](#164-immutability-aware-caching-normative)

- [17. Gestion des versions et garanties de durabilité](#17-versioning-and-durability-guarantees)

- [18. Considérations relatives à la sécurité](#18-security-considerations)

- [19. Vecteurs de test de conformité](#19-conformance-test-vectors)

- [20. Considérations relatives à l'IANA](#20-iana-considerations)

- [21. Annexe CDDL complète](#21-complete-cddl-appendix)

  - [21.1 Grammaire de séquence](#211-sequence-grammar)

  - [21.2 CDDL copiable](#212-copyable-cddl)

- [22. Pré-images de hachage, de signature et de clé d'extension](#22-hash-signature-and-extension-key-preimages)

  - [22.1 Table des pré-images et des sujets](#221-preimage-and-subject-table)

  - [22.2 Comportement des clés d'extension inconnues](#222-unknown-extension-key-behavior)

- [23. Références](#23-references)

<a id="1-overview-and-non-goals"></a>

## 1. Aperçu et non-objectifs

GTS encode un graphe sous la forme d'un journal de trames (frames) CBOR à ajout uniquement (append-only). Le graphe logique est le repli (fold) (relecture) du journal. La croissance est un ajout; la « suppression » est une suppression (suppression) logique, jamais un retrait physique; l'optimisation est une compaction séparée, explicitement avec perte (lossy), qui réécrit le journal dans un instantané (snapshot).

Quatre propriétés définissent le format :

1. **CBOR de bout en bout** (RFC 8949). Un encodage binaire ubiquitaire, normalisé par l'IETF, avec des chaînes d'octets natives (pas de taxe base64), un encodage déterministe (hachages de contenu propres) et des séquences CBOR — des éléments de données concaténés sans longueur englobante, rendant l'ajout peu coûteux. Un lecteur (reader) n'a besoin que d'une bibliothèque CBOR.

2. **Un catalogue de transformations durable.** La charge utile (payload) de chaque trame transporte une chaîne empilable de codecs provenant d'un catalogue ouvert et pérenne (`identity`, `base64`, `base85`, `gzip`, `zstd`, `lzma2`, `cose-encrypt`, …). Le catalogue sépare la durabilité de la structure (CBOR + cette spécification, pour toujours) de la densité et de la confidentialité (codecs interchangeables).

3. **Intégrité par construction.** Chaque trame (frame) porte un auto-hachage (self-hash) BLAKE3 indépendant (un identifiant de contenu) et nomme l'identifiant de son prédécesseur — une chaîne adressée par contenu de style git. La vérification est parallèle, une trame endommagée est détectable indépendamment (et les survivantes récupérables moyennant un index intact, §9.1), et l'identifiant de tête (head id) s'engage transitivement sur tout l'historique. Les signatures cryptographiques et le chiffrement (COSE, RFC 9052) sont facultatifs, superposés et agnostiques quant aux algorithmes (algorithm-agile).

4. **Composition récursive (matriochka).** Une charge utile (payload), une fois ses transformations inversées, n'est que des octets — et un fichier GTS n'est que des octets. Ainsi, une charge utile PEUT (MAY) être elle-même un GTS complet, enveloppée dans n'importe quelle transformation (compressée ou chiffrée). Un graphe signé complet peut se trouver à l'intérieur d'un champ chiffré, avec ses propres en-têtes, chaîne et signatures indépendants (§12.1).

**Non-objectifs.** GTS ne définit pas de langage de requête, de format d'index obligatoire pour la lecture, de raisonneur ou de protocole de mutation. La requête à accès aléatoire, la traversée profonde et SPARQL sont du ressort d'une cible de transformation (transform target), pas de GTS.

**Motivation informative.** GTS maintient la surface de base du lecteur (reader) à un niveau réduit : un lecteur a besoin de CBOR, BLAKE3, des codecs obligatoires et des règles de repli (fold) plutôt que d'un analyseur de texte RDF. Les outils qui nécessitent des requêtes, une indexation ou des analyses plus riches projettent les données repliées (folded) vers un substrat d'exploitation tel que N-Quads, SQLite, DuckDB ou Parquet.

<a id="2-terminology-and-conformance"></a>

## 2. Terminologie et conformité

Les mots-clés **DOIT (MUST)**, **NE DOIT PAS (MUST NOT)**, **REQUIS (REQUIRED)**, **DEVRA (SHALL)**, **DEVRAIT (SHOULD)**, **PEUT (MAY)** et **FACULTATIF (OPTIONAL)** doivent être interprétés comme décrit dans le BCP 14 (RFC 2119, RFC 8174).

- **Journal** — la séquence ordonnée de trames dans un fichier GTS.

- **Trame** — un élément de données CBOR dans le journal (§6).

- **Repli** — la relecture déterministe du journal en un état de graphe (§7.5).

- **Terme** — un terme RDF (IRI, littéral, nœud vierge ou triplet cité) avec un identifiant entier stable.

- **Réificateur** — un terme qui désigne un triplet cité, portant des métadonnées au niveau de l'énoncé (RDF 1.2).

- **Capacité** — ce qu'un lecteur doit posséder pour décoder une charge utile : une *bibliothèque de codec* ou une *clé*.

- **Nœud opaque** — la représentation graphique d'une trame que le lecteur n'a pas pu décoder (§7.6).

<a id="21-conformance-scopes"></a>

### 2.1 Portées de conformité

Cette spécification sépare les portées de conformité suivantes :

- **Conformité au format de transmission** couvre la structure de séquence CBOR au niveau des octets, l'encodage CBOR déterministe, la grammaire des en-têtes et des trames, les préimages de content-id, et les frontières de segment.

- **Conformité du lecteur** couvre l'analyse, la vérification de la chaîne, la résolution de la charge utile, le comportement de repli, les diagnostics, la gestion des noeuds opaques, et le comportement lié aux limites de ressources.

- **Conformité du rédacteur** couvre la production d'une sortie déterministe, des en-têtes et des trames valides, des identifiants de contenu corrects, des déclarations de codec, et des préimages de signature/hachage.

- **Conformité de l'outil** couvre la politique de la ligne de commande ou de la bibliothèque qui est plus stricte que la validité du fichier local, comme la validation des opérations de composition, d'extraction, de publication ou d'archivage.

- **Conformité du profil** couvre le vocabulaire, la validation, la capacité et les règles de confiance spécifiques au profil, superposés au format central.

- **Conformité du déploiement** couvre le comportement de service et de distribution tel que le type de média, la mise en cache, les requêtes de plage, et la préservation des octets sur HTTP ou l'hébergement d'artefacts.

Les classes de conformité ci-dessous définissent le comportement du lecteur et du rédacteur. Les exigences relatives aux outils, aux profils et au déploiement sont délimitées explicitement dans les sections qui les définissent.

La conformité de base du lecteur/rédacteur est indépendante de la validation du profil, des verbes de la CLI, des cibles de transformation, et du comportement de déploiement HTTP. Un fichier GTS localement valide reste localement valide lorsqu'il déclare un profil non pris en charge ; un lecteur enregistre la déclaration de profil et replie les octets selon sa classe de lecteur, tandis qu'un outil soucieux du profil PEUT (MAY) appliquer des vérifications supplémentaires dans la portée de la conformité du profil.

Les profils, les outils et les déploiements NE DOIVENT PAS (MUST NOT) modifier la grammaire des en-têtes ou des trames, la détection des frontières de segment, les préimages de content-id ou de signature/hachage, la résolution du catalogue de transformations, ou la sémantique de repli centrale au §7. Un profil plus strict PEUT (MAY) rejeter un artefact autrement valide uniquement en tant qu'échec de validation au niveau du profil, et non en redéfinissant la validité GTS centrale.

<a id="22-reader-and-writer-conformance-classes"></a>

### 2.2 Classes de conformité du lecteur et du rédacteur

- Un **lecteur de base (Baseline Reader)** DOIT (MUST) : analyser la séquence CBOR ; vérifier la chaîne id/prev (§9.1) ; effectuer le repli (fold) des trames `terms`,
  `quads`, `reifies`, `annot`, `blob`, `suppress`, `meta` et `snapshot` ; prendre en charge les
  codecs `identity`, `gzip` et `zstd` ; et exposer toute trame qu'il ne peut pas décoder comme un noeud opaque
  (§7.6). Il PEUT (MAY) ignorer les signatures et le chiffrement.

- Un **lecteur en continu (Streaming Reader)** est un lecteur de base (Baseline Reader) qui traite les trames une à la fois et les émet vers un
  puits **sans matérialiser l'ensemble du graphe** : il ne conserve que le dictionnaire de termes (et une
  vérification de chaîne en cours), ainsi que la taille maximale de trame décodée et l'état du sidecar de validation, ce qui donne une mémoire conservée de
  O(termes distincts + taille maximale de trame décodée + état du sidecar de validation)
  plutôt que O(triplets + blobs) (§7.7). Les transformations `gts → duckdb`/`sqlite` (§14) adoptent le profil d'un
  lecteur en continu (Streaming Reader) lorsqu'elles sont implémentées via un puits sans matérialisation.

- Un **lecteur complet (Full Reader)** vérifie de plus les signatures COSE, déchiffre les trames chiffrées par COSE pour
  lesquelles il détient les clés, PEUT (MAY) effectuer une récursion dans les blobs GTS imbriqués (§12.1), et PEUT (MAY) utiliser la trame d'index
  optionnelle (§6.2) pour la vérification parallèle et l'accès aléatoire.

- Un **rédacteur (Writer)** DOIT (MUST) émettre du CBOR déterministe (§4) pour tous les octets hachés ou signés, et
  DOIT (MUST) calculer l'auto-hachage `"id"` de chaque trame et définir `"prev"` sur le `"id"` de l'élément précédent.

<a id="23-baseline-reader-api-shape"></a>

### 2.3 Forme de l'API du lecteur de base

Un lecteur de base DEVRAIT (SHOULD) exposer au moins :

```text
open(bytes|path)            -> Graph          # parse + verify chain + fold
Graph.quads()               -> iterator[(s,p,o,g)]   # term ids resolved to terms
Graph.term(id)              -> Term
Graph.annotations(reifier)  -> iterator[(prop, value)]
Graph.blob(digest)          -> bytes | OpaqueRef
Graph.opaque()              -> iterator[OpaqueNode]
Graph.to_nquads(out)        # §14
```

Cette forme d'API est intentionnellement restreinte : elle expose les tables repliées, les diagnostics et le chemin de projection commun sans nécessiter d'analyseur de texte RDF, de résolveur de préfixes, de moteur de requête ou de raisonneur.
Le contrat de parité entre l'API multi-langage et l'interface de ligne de commande (CLI) est maintenu dans
[`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md).

<a id="24-reader-diagnostics"></a>

### 2.4 Diagnostics du lecteur

Les lecteurs exposent des diagnostics observables par machine avec ces classes canoniques (une implémentation PEUT (MAY) les mapper à des retours d'erreur ou à des avertissements structurés) :

| classe | signification |

|---|---|

| `EmptyFile` | flux d'octets vide ou aucun en-tête de segment ; retourner un résultat vide avec un diagnostic fatal plutôt que d'abandonner (§3) |

| `TornAppendError` | élément CBOR incomplet à la fin du fichier (EOF) (§3) |

| `DamagedFrame` | incohérence self-`"id"` / hachage de trame invalide (corruption du contenu) ; `reason:"damaged"` opaque (§7.6) |

| `BrokenChain` | hachage de trame valide, mais `"prev"` ≠ `"id"` de l'élément précédent (insertion / réordonnancement / épissure) (§9.1) |

| `TruncatedLog` | un engagement de tête est présent mais la tête observée diffère (§9, §18) |

| `UnknownCodec` | une transformation nomme un codec qui manque au lecteur ; `reason:"unknown-codec"` opaque |

| `MissingKey` | un codec `encrypt` que le lecteur ne peut pas décrypter ; `reason:"missing-key"` opaque |

| `KeyWrapFailed` | l'ouverture d'une clé multi-destinataire différée a échoué ; `reason:"missing-key"` opaque |

| `ConflictingReifier` | un réificateur s'est lié à un triplet différent (§7.8) |

| `PositionConstraint` | un terme apparaît dans une position illégale de sujet, prédicat, objet ou nom de graphe ; rejeter/diagnostiquer la ligne fautive (§7.4) |

| `ForwardReference` | une référence d'identifiant de terme nomme un terme non introduit par une trame antérieure dans le même segment (§7.2, §7.5) |

| `SegmentBoundary` | un lecteur de compatibilité atteint un en-tête de segment ultérieur où les identifiants de termes globaux au fichier se replieraient mal ; s'arrêter avec un diagnostic fatal (§3.1, §19) |

| `IllTypedLiteral` | un littéral de type de données XSD reconnu a une forme lexicale invalide ; préserver le littéral textuellement et exposer un indicateur de diagnostic/métadonnées (§7.1) |

| `RecursionLimit` | profondeur de GTS imbriqué ou budget de taille décodée dépassé (§12.1, §18) |

| `StreamableLayoutError` | un segment revendique `"layout": "streamable"` mais sa région couverte viole l'ordre de livraison, ou son pied de page d'index est manquant ou contredit les trames qu'il couvre (§3.3) |

| `IndexMmrError` | une racine `index.mmr` optionnelle est présente mais ne correspond pas aux identifiants de trame couverts (§6.2) |

| `UnknownFrameType` | un type de trame n'est pas compris par le lecteur/profil ; préserver la vérification de la chaîne et soit l'ignorer, soit l'exposer comme opaque jusqu'à ce qu'un profil le gère (§7.8) |

<a id="3-file-structure"></a>

## 3. Structure du fichier

Un fichier GTS est une **CBOR Sequence** (RFC 8742) : zéro octet de cadrage entre les éléments, chaque élément étant un élément de données CBOR bien formé. Chaque segment PEUT (MAY) commencer par l'étiquette d'auto-description CBOR `55799` (`0xd9 0xd9 0xf7`) comme nombre magique pour ce segment. Si elle est présente, l'étiquette `55799` DOIT (MUST) étiqueter l'élément de données **Header** du segment ; il ne s'agit pas d'un élément de journal distinct, il n'a pas de `"id"` et ne participe pas à la chaîne id/prev. Un fichier GTS NE DOIT PAS (MUST NOT) envelopper l'ensemble de la séquence dans un élément CBOR externe.

```text
GTS-file = segment *segment
segment  = [self-describe-tag] header *frame
```

- Le **premier** élément de données d'un segment DOIT (MUST) être un **Header** (§5).

- Chaque élément de données subséquent d'un segment est une **Frame** (trame) (§6), dans l'ordre du journal, jusqu'au prochain Header (qui commence un nouveau segment) ou jusqu'à la fin de l'entrée.

- **Append** = concaténer une trame supplémentaire (prolongeant le dernier segment), ou concaténer un segment complet supplémentaire (§3.1). Aucun préfixe de longueur ou compte n'est stocké, de sorte qu'un rédacteur (writer) ne réécrit jamais les octets précédents.

<a id="31-multi-segment-files-cat-append-composition"></a>

### 3.1 Fichiers multi-segments (`cat`)

Un fichier GTS est constitué d'un ou plusieurs **segments**, chacun étant un journal
`header *frame` complet et autonome. La propriété déterminante : **la concaténation d'octets de fichiers GTS valides est un fichier GTS valide** —

```sh
cat music.gts >> core.gts        # core.gts is now a valid two-segment GTS
```

- **Détection des limites (normative).** Un lecteur (reader) qui a consommé au moins une trame (frame) et
  rencontre un élément de données qui est une carte (map) contenant la clé `"gts"` et dépourvu de la clé `"t"`
  DOIT (MUST) le traiter comme l'En-tête (Header) d'un **nouveau segment** (l'étiquette d'auto-description facultative `55799`
  PEUT (MAY) étiqueter cet en-tête ; les rédacteurs (writers) DEVRAIENT (SHOULD) émettre l'étiquette sur chaque en-tête de segment pour rendre les limites
  reconnaissables par l'humain). L'étiquette est attachée à l'en-tête du segment, de sorte que la concaténation d'octets de
  segments étiquetés indépendamment valides reste une concaténation d'octets d'éléments de séquence CBOR, et non un
  wrapper de fichier complet imbriqué. Tout autre élément qui n'est pas une trame reste une entrée malformée (§17).

- **Intégrité indépendante.** Chaque segment possède sa propre genèse (son en-tête `"id"`), sa propre
  chaîne id/prev, ses propres signatures et sa propre `index` facultative (un index ne couvre QUE son
  segment). L'identité composite du fichier est la **liste ordonnée des identifiants de tête de segment**. Un
  segment tiers porte son propre signataire ; la concaténation ne réécrit rien (un `cat` ne peut pas
  réécrire l'en-tête d'un segment précédent sans briser son auto-hachage — par conception).

- **Identité entre les segments.** Les term-ids sont **de portée segment** (§7.2) ; la SEULE identité entre segments
  est la **valeur** du terme (IRI, littéral, structure de triple cité). Les étiquettes de nœuds vierges (blank-node labels) sont
  locales au segment et NE DOIVENT PAS (MUST NOT) être fusionnées entre les segments (la règle GTS imbriqué de §12.1, appliquée au
  niveau supérieur).

- **Union des profils.** L'ensemble effectif de profils (profiles)/exigences du fichier est l'union des valeurs `"prof"`
  des en-têtes de segment (et de toutes les exigences de profil portées dans les métadonnées de segment). Un lecteur (reader)
  ne disposant pas des capacités requises par un segment dégrade les trames (frames) de ce segment en nœuds opaques
  (§7.6) — « ces données nécessitent le profil gmeow-music » est une lecture d'en-tête, pas une erreur.

- **Relation avec l'imbrication.** Le GTS imbriqué (§12.1) se compose par *confinement* (un sous-graphe scellé et
  expédiable indépendamment) ; les segments se composent par *concaténation* (agrégation ouverte sans outil).
  Les deux produisent un repli d'union (union fold) ; choisissez l'imbrication lorsque la partie doit voyager ou être scellée
  indépendamment, et les segments lorsque le simple `cat` doit fonctionner.

<a id="32-streaming-and-progressive-enhancement"></a>

### 3.2 Diffusion en continu et amélioration progressive

Le journal en ajout uniquement fait de la diffusion en continu une **propriété du format**, et non une fonctionnalité d'un outil.
Trois faits se composent, et les mises en œuvre conformes DOIVENT (MUST) préserver les trois :

- **Validité du repli de préfixe (normative).** Chaque préfixe d'octets d'un fichier GTS valide qui se termine sur une limite d'élément de données est lui-même un fichier GTS valide, et un lecteur DOIT (MUST) le replier exactement dans l'état qu'il atteindrait en repliant ces mêmes éléments à l'intérieur du fichier complet. Un flux en direct en cours de transmission est donc *indistinguable* d'un fichier avec un ajout tronqué (§3) : l'élément de fin partiel signifie « pas encore arrivé », et un consommateur PEUT (MAY) continuer la lecture à mesure que les octets arrivent (sémantique `tail -f`) — chaque repli intermédiaire est un état de graphe réel et utilisable, jamais un état d'erreur à moitié analysé.

- **Raffinement monotone.** Les trames ajoutées ne font qu'*ajouter* de la connaissance : les quads s'accumulent (sémantique d'ensemble §7.8), une liaison de réificateur est « premier arrivant, premier servi » de sorte qu'un rendu établi ne change jamais sous celui-ci, et la suppression est une superposition d'affichage additive (§11) — l'arrivée d'une trame `suppress` affine la présentation sans invalider aucun repli antérieur. La vérification de la chaîne est également incrémentale : l'état O(1) (le `"prev"` attendu) vérifie chaque trame à mesure qu'elle arrive.

- **Cadrage sécurisé par blocs.** Les éléments de séquence CBOR sont auto-délimitants, de sorte que les limites d'éléments sont des points de re-découpage sécurisés pour les relais et les mandataires (proxies), et la reprise est adressée par le contenu : un récepteur qui indique la dernière trame `"id"` qu'il a vérifiée peut reprendre à partir de l'octet suivant sans aucune négociation au-delà de ce hachage.

**Amélioration progressive.** Les producteurs DEVRAIENT (SHOULD) ordonner le contenu du plus significatif au moins significatif afin qu'un préfixe précoce soit maximalement utile : à l'intérieur d'un segment, `terms`/`quads` (le graphe) avant les trames `blob` volumineuses, et les petites manifestations ou aperçus avant les grandes ; à travers un fichier, les segments SONT les couches d'amélioration — un segment de base (graphe central + vignettes) suivi de segments d'amélioration (blobs haute résolution, projections calculées) donne au récepteur un paquet complet et vérifiable à chaque limite de segment, les règles de composition du §3.1 étant appliquées comme un calendrier de livraison.
**Les trames de point de contrôle `index`** (§6.2) émises périodiquement donnent à un consommateur de flux des ancres de troncature intermédiaires (`"head"`), des décalages d'accès aléatoire pour une récupération par plage (ranged re-fetch), et un manifeste de ce qui est arrivé ; l'index demeure un accélérateur, jamais une dépendance (§3, §6.2).

**Le manifeste est le graphe.** GTS n'a pas besoin de structure de table des matières, car les trames qui *décrivent* le contenu peuvent précéder les trames qui le *transportent* : un producteur DEVRAIT (SHOULD) émettre les quads nommant chaque manifestation à venir — son condensé de contenu, son type de média, sa taille, son rôle — avant les trames `blob` dont elles promettent les octets. Le repli d'un préfixe précoce contient alors le calendrier de livraison comme une connaissance ordinaire : chaque condensé que le graphe nomme mais que le flux n'a pas encore livré est une reconnaissance de dette (IOU) adressée par le contenu, ainsi « s'arrêter ici », « sauter en avant » et « récupérer par plage uniquement le fichier RAW » sont des décisions *éclairées* du consommateur, prises par rapport à un catalogue vérifiable plutôt que par intuition. (Un blob qui n'arrive jamais dans ce fichier est simplement un blob externe, §12 — la référence se dégrade gracieusement vers « les octets résident ailleurs ».)

*Calendrier de livraison concret* — une photographie sous forme de flux progressif ; un consommateur peut s'arrêter à n'importe quelle limite d'élément avec un paquet complet et vérifié de tout ce qui se trouve au-dessus de son point d'arrêt :

```text
header                          profile, codec catalog
terms/quads                     the catalog: Work + every manifestation below,
                                each with digest, mt, size, role (the IOUs)
blob  image/webp        ~20 KB  thumbnail — first paint
blob  image/jxl         ~8 MB   full-resolution render
terms/quads                     scene description (what is IN the image)
blob  image/x-raw       ~80 MB  RAW sensor dump
meta/quads                      full camera metadata
terms/quads/annot               AI analysis as RDF, statement-level provenance
terms/quads/annot               opinions — standpoint-qualified claims
terms/quads                     processing-pipeline provenance
index                           footer: offsets, head anchor, MMR (§6.2)
```

Un spectateur occasionnel s'arrête après la vignette ; un archiviste prend tout ; un éditeur effectue une récupération par plage du RAW par condensé après avoir lu uniquement le catalogue. Mêmes octets, même chaîne, trois consommateurs.
Un lecteur diffuse en continu les éléments jusqu'à la fin de l'entrée. Les octets partiels de fin (un ajout tronqué) DOIVENT (MUST) être détectés et ignorés avec un diagnostic : un lecteur tente de décoder chaque élément CBOR successif, et si le décodeur signale un élément incomplet ou une fin de fichier (EOF) inattendue en fin d'entrée, il DOIT (MUST) traiter les octets de fin comme un ajout tronqué, ignorer cet élément incomplet et fournir un diagnostic observable par machine (par exemple, un avertissement `TornAppendError`). En particulier, si une panne est survenue lors de l'écriture d'une trame `index` (§6.2), l'index de fin est tronqué : un lecteur DOIT (MUST) l'ignorer et se rabattre sur une `index` intacte antérieure ou sur un simple **balayage séquentiel**, afin que chaque trame survivante reste récupérable. L'index optionnel est un accélérateur, jamais une dépendance.

Chaque propriété ci-dessus est valable pour n'importe quel ordre de trame ; ce qu'un producteur *choisit* comme ordre est une préoccupation distincte et nommée : un segment se trouve dans l'un des deux **états de disposition** — **accrétive** (ordonnée par ajout) ou **diffusable en continu** (ordonnée par livraison) — définis ci-après (§3.3).

<a id="33-layout-states-accretive-and-streamable"></a>

### 3.3 États de disposition : accrétif et diffusable en continu

Un segment GTS est toujours valide et toujours repliable par préfixe (§3.2), mais il réside dans l'un des deux états de disposition :

- **Accrétif** — optimisé pour l'ajout. Les trames arrivent dans l'ordre d'arrivée (capture en direct, accumulation en mémoire d'agent, accumulation de preuves). Les écritures sont peu coûteuses à jamais et le flux est consommable au fur et à mesure qu'il arrive, mais l'importance n'est pas chargée à l'avant et le catalogue peut suivre les octets qu'il décrit. C'est l'état par défaut ; il n'est jamais déclaré.

- **Diffusable en continu** — ordonné pour la livraison. Le catalogue présage la charge utile : un index de diffusion de tête (trames `terms`/`quads` ordinaires dans le vocabulaire `stream`, §13.3 — un `stream:Manifestation` par blob promis, portant l'empreinte numérique [digest], le type de média, la taille, le rôle et l'ordre prévu) précède chaque trame `blob`, les blobs suivent par ordre de plus grande importance en premier, et un décalage de fin `index` (§6.2) ferme la région couverte en tant que pied de page à accès aléatoire.

L'ajout convivial et l'optimal pour la diffusion sont des dispositions différentes du même contenu (précédent : mp4 `faststart`, réécritures du répertoire central zip, compactage LSM). Un rédacteur (writer) à passage unique ne peut pas produire le second état, la conversion est donc une réécriture explicite — **compactage diffusable en continu** (§10.1), exposé comme `gts compact --streamable` (§14.1).

**La revendication (normatif).** Un segment déclare l'état diffusable en continu avec la clé d'en-tête optionnelle `"layout": "streamable"` (§5). La revendication est par segment (chaque segment a son propre en-tête, §3.1) et inviolable (l'auto-hachage de l'en-tête la couvre). La diffusabilité en continu est une **revendication déclarée vs calculée** au sens du §14.1 — refusez-ne-faites-pas-confiance :

- La **région couverte** d'un segment revendiqué est le préfixe délimité par la **dernière trame `index` intacte** du segment : des trames `"count"`, se terminant à la trame dont le `"id"` est égal au `"head"` de l'index. Le pied de page DOIT (MUST) suivre immédiatement les trames qu'il couvre (`"count"` = la position de la propre trame de l'index − 1) — sinon des trames pourraient se trouver entre le préfixe couvert et le pied de page, n'étant comptées ni comme couvertes ni comme queue accrétive. Un segment revendiqué sans trame `index` intacte, dont le dernier index n'est pas immédiatement adjacent à son préfixe couvert, ou dont le `"head"` n'est pas égal à l'identifiant de la trame `"count"`, est en violation.

- À l'intérieur de la région couverte, chaque trame `blob` en ligne DOIT (MUST) être précédée d'une trame `quads` qui décrit son empreinte numérique [digest] via `stream:digest` (§13.3) — catalogue-avant-charge-utile. Un blob couvert livré avant sa description est en violation.

- Un lecteur (reader) rencontrant une violation DOIT (MUST) faire remonter un diagnostic **`StreamableLayoutError`** (§2.3) ; un outil de vérification le traite comme une erreur (§14.1). La revendication ne peut jamais se corrompre par rapport aux octets.

**Les ajouts après compactage sont légaux et repliables.** Les trames après le dernier `index` sont simplement *non présagées* : elles constituent la **queue accrétive** du segment, ne comportent aucune obligation d'ordonnancement et ne déclenchent aucun diagnostic. Le segment est alors « diffusable en continu jusqu'à la trame *N*, accrétif après » — l'outillage DEVRAIT (SHOULD) signaler la limite (§14.1). Re-compactez pour re-diffuser efficacement. De même, un segment ajouté par `cat` ne fait aucune revendication à moins que son propre en-tête n'en fasse une.

**Préfixes en vol.** Un préfixe d'un segment diffusable en continu coupé avant le `index` de fin a, par construction, une revendication et pas encore de pied de page ; un consommateur en continu NE DOIT PAS (MUST NOT) traiter le pied de page manquant comme un mensonge tant que l'entrée peut encore arriver — la violation de pied de page manquant s'applique à un fichier *complet*. La règle du catalogue-avant-charge-utile, par contre, est stable par préfixe : une violation observée dans n'importe quel préfixe est une violation du fichier entier.

<a id="4-cbor-conventions"></a>

## 4. Conventions CBOR

- Les mappages utilisent des **clés sous forme de chaînes de texte courtes** (p. ex. `"t"`, `"d"`) pour l'auto-description et le débogage visuel ; la compacité est la tâche de la couche de transformation, pas celle du schéma.

- Tous les octets qui sont **hachés ou signés** DOIVENT (MUST) utiliser un **Codage déterministe** (RFC 8949 §4.2) : entiers sous la forme la plus courte, éléments de longueur définie, et clés de mappage triées **octet par octet selon leur forme codée** — explicitement la règle de la RFC 8949, et NON l'ordonnancement canonique de la RFC 7049 basé d'abord sur la longueur. (Pour les clés de texte courtes que GTS utilise lui-même, les deux coïncident, car l'octet initial d'une chaîne de texte CBOR intègre sa longueur ; les règles divergent pour les clés de types mixtes, ainsi les mises en œuvre NE DOIVENT PAS (MUST NOT) se fier au mode « canonique » hérité d'une bibliothèque CBOR sans vérifier quel ordonnancement elle met en œuvre.)

- Des entiers non signés sont utilisés pour tous les ids. Les condensés BLAKE3 sont des chaînes d'octets de 32 octets (256 bits).

- De courts fragments de grammaire sont fournis en **CDDL** (RFC 8610). L'annexe CDDL complète pouvant être copiée est au §21, et les règles de pré-image canoniques sont au §22.

```cddl
term-id      = uint            ; append-order, frozen (§7.2)
digest       = bstr .size 32   ; BLAKE3-256
content-id   = digest          ; a frame's self-hash (§9.1)
digest-ref   = digest / tstr    ; raw digest or "blake3:<hex>" text (§21.2)
codec-id     = uint            ; index into the header codec catalog (§8)
```

<a id="5-header"></a>

## 5. Header

Le Header est le premier élément de données et la genèse de la chaîne ; ce n'est pas une trame (il n'a pas de `"prev"`).

```cddl
header = {
  "gts"  : "GTS1",                    ; magic / format id
  "v"    : uint,                      ; spec major version (1)
  "prof" : tstr,                      ; profile (§13); "generic" if unspecified
  "cat"  : { * codec-id => codec },   ; the transform catalog (§8)
  ? "layout": tstr,                   ; layout-state claim (§3.3); absent = accretive
  ? "dct": { * tstr => bstr },        ; named, UNCOMPRESSED dictionaries for dict-codecs
  ? "meta": any,                      ; free-form, non-normative metadata
  "id"   : content-id,                ; self-hash of the header content (the chain genesis)
}

codec = {
  "name" : tstr,                      ; "identity" | "gzip" | "zstd" | "lzma2" | "cose-encrypt" | ...
  "cls"  : "encode" / "compress" / "encrypt",
  ? "dct": tstr,                      ; references header "dct" key (dict codecs)
  ? "p"  : any,                       ; codec parameters (e.g. lzma2 level)
}
```

Le catalogue est **fermé au sein d'un fichier** (une trame ne peut référencer que les codec-ids déclarés par le header) mais **ouvert à travers l'écosystème** (de nouveaux codecs peuvent être enregistrés par nom). Le Header porte son propre `"id"` (auto-hachage de son contenu) et aucun `"prev"` — il est la genèse, et le `"prev"` de la première trame est le `"id"` du Header. Le `"id"` du Header DOIT (MUST) être égal au BLAKE3-256 du CBOR déterministe de la carte du Header **en excluant la clé `"id"`** ; toutes les autres clés (y compris `"meta"` et les clés d'extension inconnues) y participent. La table de préimage au §22 est l'unique source de vérité pour les octets hachés et signés. La clé facultative `"layout"` revendique un état de disposition (§3.3) : la seule valeur définie par cette révision est `"streamable"`, qu'un lecteur de vérification DOIT (MUST) vérifier par rapport à la disposition réelle du segment ; les lecteurs DOIVENT (MUST) ignorer les valeurs `"layout"` inconnues (compatibilité ascendante — un état inconnu n'impose aucune vérification). Les dictionnaires sont stockés **non compressés et intrabande** — il n'y a pas de dépendance à un dictionnaire externe. La valeur `"dct"` d'un codec DOIT (MUST) correspondre à une clé dans la carte `"dct"` du header, et le codec DOIT (MUST) utiliser la chaîne d'octets correspondante comme dictionnaire de compression/encodage.

<a id="6-frames"></a>

## 6. Trames

Toutes les trames partagent une enveloppe :

```cddl
frame = {
  "t"   : frame-type,        ; discriminator
  ? "x" : [+ codec-id],      ; transform chain, applied in order on encode; default [identity]
  ? "pub": any,              ; CLEARTEXT public envelope (always readable; §9.4)
  ? "to": [+ recipient],     ; recipients, for encrypt-class chains
  ? "d" : bstr / any,        ; payload: bstr when "x" transforms it; structured CBOR otherwise
  "prev": content-id,        ; the PREVIOUS data item's "id" (chain link; §9.1)
  "id"  : content-id,        ; BLAKE3-256 self-hash of this frame's CONTENT (all keys but "id"/"sig")
    ? "sig": bstr,           ; COSE_Sign1 over "id" (§9.2)
}

frame-type = "terms" / "quads" / "reifies" / "annot" / "blob" / "suppress"
/ "snapshot" / "meta" / "index" / "opaque"

recipient = { "kid": tstr, ? "alg": tstr, * tstr => any }   ; key identifier; never the key
```

Le `"id"` de chaque trame DOIT (MUST) être égal au BLAKE3-256 du CBOR déterministe de son contenu (chaque clé à l'exception de `"id"` et `"sig"` ; les clés d'extension inconnues participent). Le `"prev"` de chaque trame DOIT (MUST) être égal au `"id"` de l'élément de données précédent ; le `"prev"` de la **première** trame est le `"id"` de l'En-tête (Header).
Puisque `"prev"` se trouve à l'intérieur du contenu haché, chaque `"id"` s'engage de manière transitive envers toutes les trames précédentes (§9.1). Le §22 centralise l'ensemble des règles relatives à l'image pré-hachage (preimage) et au sujet.

<a id="61-payload-resolution"></a>

### 6.1 Résolution de la charge utile

Pour obtenir la charge utile logique d'une trame :

1. Si `"x"` est absent, la charge utile est `"d"` directement (CBOR structuré) — ce qui équivaut à une seule transformation `identity` ; une chaîne se résolvant uniquement par `identity` laisse de même `"d"` inchangé.

2. Si `"x"` est présent, `"d"` DOIT (MUST) être une chaîne d'octets et chaque codec-id DOIT (MUST) se résoudre via l'en-tête `"cat"` ; appliquer l'**inverse** de chaque codec, du dernier au premier. Chaque étape nécessite une **capacité** (§8.3). En cas de capacité manquante (codec inconnu ou clé manquante), s'arrêter et traiter la trame comme **opaque** (§7.6).

3. Les octets entièrement décodés sont un élément CBOR ; les décoder selon la structure spécifique au type (§7).

<a id="62-index-frame-optional"></a>

### 6.2 Trame d'index (facultatif)

Un rédacteur PEUT (MAY) ajouter une trame `index` — un pied de page qui accélère les fichiers volumineux sans élever le seuil minimal du lecteur simple (un lecteur de base (Baseline Reader) l'ignore). Étant donné que le journal est en ajout uniquement (append-only), une nouvelle trame `index` PEUT (MAY) être ajoutée après d'autres trames ; la **dernière** `index` l'emporte.

```cddl
index-payload = {
  "count"  : uint,                        ; frames covered
  "head"   : content-id,                  ; "id" of the last covered frame (truncation anchor)
  ? "off"  : [+ uint],                    ; byte offset of each frame (random access; parallel verify)
  ? "ti"   : { * frame-type => [+ uint] },; frame indices by type
  ? "dict" : [+ uint],                    ; indices of "terms" frames (dictionary locator; §7.7)
  ? "mmr"  : content-id,                  ; Merkle-Mountain-Range root over frame ids (§9.1)
}
```

Étant donné `"off"`, un lecteur complet (Full Reader) répartit la vérification des hachages de trames entre les fils d'exécution (threads) et effectue un positionnement (seek) vers n'importe quelle trame ; étant donné `"dict"`, un lecteur en continu (Streaming Reader) ne charge que le dictionnaire (§7.7) ; étant donné `"head"`/`"mmr"`, il détecte la troncature et produit des preuves d'inclusion en O(log n). Un index de **point de contrôle** (checkpoint) est simplement une trame `index` émise périodiquement plutôt que seulement en tant que pied de page ; une trame `index` antérieure PEUT (MAY) encore servir d'ancrage de récupération même si la dernière trame `index` intacte est préférée pour l'accélération. Le support actuel des paquets et les reports pour `off`/`ti`, `dict`, `mmr`, les verbes de preuve, la récupération de plage (range fetch) et les flux de travail de réplication sont suivis dans [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md).

La racine `mmr` utilise une structure Merkle-Mountain-Range sur les identifiants de trame ordonnés couverts par l'index. La trame d'index elle-même n'est pas couverte par sa propre `mmr` ; un index ultérieur peut couvrir une trame d'index antérieure comme toute autre trame antérieure. Les pics sont construits de gauche à droite en ajoutant une feuille par identifiant de trame et en fusionnant de manière répétée les deux pics les plus à droite tant que leurs hauteurs correspondent. La racine pour `count = 0` est la préimage de racine avec une liste de pics vide.

Toutes les préimages MMR utilisent le format CBOR (§4) déterministe et BLAKE3-256 :

```text
leaf(index, frame_id) =
  BLAKE3-256(deterministic-CBOR(["gts-mmr-leaf-v1", index, frame_id]))

parent(parent_height, left_hash, right_hash) =
  BLAKE3-256(deterministic-CBOR(["gts-mmr-parent-v1",
                                 parent_height, left_hash, right_hash]))

root(count, peaks) =
  BLAKE3-256(deterministic-CBOR(["gts-mmr-root-v1",
                                 count,
                                 [[peak_height, peak_hash], ...]]))
```

Un objet JSON de preuve détachée a cette forme stable :

```json
{
  "schema": "gts-mmr-proof-v1",
  "hash": "blake3-256",
  "preimage": "gts-mmr-v1",
  "count": 4,
  "leaf_index": 2,
  "frame_id": "<32-byte frame id hex>",
  "root": "<32-byte mmr root hex>",
  "peak_index": 0,
  "peaks": [{"height": 2, "hash": "<32-byte peak hex>"}],
  "path": [
    {"side": "right", "parent_height": 1, "hash": "<32-byte sibling hex>"}
  ]
}
```

`verify-proof` DOIT (MUST) rejeter une preuve à moins que les hauteurs de pics ne correspondent à `count`, que `leaf_index` appartienne à `peak_index`, que chaque champ de hachage fasse 32 octets, que le chemin reconstruise le pic sélectionné et que les pics reconstruisent `root`. La vérification de la preuve ne nécessite pas le fichier `.gts` d'origine.

<a id="7-graph-data-model-and-fold"></a>

## 7. Modèle de données de graphe et repli

Un repli (fold) est la projection déterministe d'un journal de trames (frame log) ordonné dans un état de graphe indexé par valeur. Les identifiants de termes sont des artefacts de compression locaux utilisés lors de la lecture d'un segment ; le graphe de fichier exposé est défini par les règles d'union de valeurs ci-dessous.

**Modèle d'état replié (normatif).** Un lecteur (reader) replie chaque segment dans cet état logique :

- `terms` : un vecteur ordonné de valeurs de termes locales au segment.

- `quads` : un ensemble de quads RDF assertés sur des valeurs de termes.

- `reifiers` : une correspondance partielle d'une valeur de terme réificateur vers exactement une valeur de triplet cité.

- `annotations` : un multi-ensemble ordonné de rangées `(reifier, predicate, value)` sur des valeurs de termes.

- `blobs` : une correspondance adressée par contenu d'un condensé BLAKE3 vers des octets en ligne ou une référence de contenu externe.

- `blob_meta` : une correspondance de métadonnées superficielle par condensé de blob, construite à partir de correspondances de blob `"pub"`.

- `meta` : une correspondance de métadonnées de segment superficielle, plus une fusion superficielle au niveau du fichier (§7.5).

- `suppressions` : une liste ordonnée de directives d'affichage/de préséance (§11).

- `opaque` : une liste ordonnée de trames (frames) transportées intentionnellement ou nécessairement sans sémantique de charge utile décodée (§7.6).

- `signatures` et `diagnostics` : des observations ordonnées du lecteur (reader) concernant les trames (frames). Elles font partie de l'état GTS replié mais ne font pas partie du jeu de données RDF.

- `segment_heads`, `segment_profiles`, `segment_meta`, et l'état de disposition du segment : le registre de segment ordonné nécessaire pour préserver l'identité de concaténation (cat-append), les exigences de profil (profile), les métadonnées par segment et les revendications de disposition diffusable en continu (streamable-layout).

**Repli de fichier (normatif).** Un repli de fichier est l'union de valeurs ordonnée de ses replis de segments : les segments sont traités dans l'ordre du fichier ; chaque identifiant de terme est d'abord résolu à l'intérieur de son propre segment ; puis les quads, les liaisons de réificateurs, les annotations, les déclarations de blobs, les métadonnées, les suppressions, les nœuds opaques (opaque nodes), les signatures, les diagnostics et les registres de segments sont fusionnés selon les règles des §7.5 et §7.8. L'union NE DOIT PAS (MUST NOT) comparer les identifiants de termes bruts entre les segments. L'identité entre segments est toujours une identité de valeur.

**Substrat RDF (normatif).** GTS importe les « RDF 1.2 Concepts and Abstract Data Model Candidate Recommendation Snapshot » datés du 07 avril 2026 pour les IRI, les nœuds vierges (blank nodes), les littéraux, les jeux de données RDF, les termes de triplets, l'étiquette de version `"1.2"`, et `rdf:reifies` (§23.1). GTS gèle ce substrat pour la version majeure 1, à moins qu'une version majeure ultérieure de GTS ne mette à jour cette référence. Un lecteur de base (Baseline Reader) n'a pas besoin d'implémenter un analyseur RDF, un langage de requête, un régime d'implication, un algorithme de canonisation ou une syntaxe concrète RDF 1.2 ; il n'a besoin que du mappage de termes, de quads, de réificateurs, d'annotations et de jeux de données défini ici. Les régimes d'implication de la sémantique RDF ne font pas partie du repli GTS principal, à moins qu'un profil (profile) ou une projection ne les applique explicitement au-dessus de la couche de transport.

**Égalité de valeurs (normative).** Le repli compare les valeurs comme suit :

| type de valeur | règle d'égalité |

|---|---|

| IRI | Égalité exacte de la chaîne Unicode après le décodage CBOR. Aucune normalisation de pourcentage, de casse, Unicode, d'IRI de base ou de préfixe n'est appliquée par le GTS de base. |

| Littéral | Même chaîne lexicale, même valeur d'IRI de type de données après l'application des valeurs par défaut (§7.1), même chaîne d'étiquette de langue lorsqu'elle est présente, et même direction de base RDF 1.2 lorsqu'elle est présente. La canonisation lexicale du type de données n'est pas appliquée ; `"01"^^xsd:int` et `"1"^^xsd:int` sont des valeurs de transport distinctes. |

| Étiquette de langue | Égalité exacte de la chaîne dans le GTS de base. Les profils et les projections PEUVENT (MAY) appliquer une correspondance de plage de langues, une casse BCP 47 préférée ou une traduction d'étiquette publique/privée (§13.1), mais cela ne constitue pas l'identité du terme. |

| Type de données | Égalité de la valeur de l'IRI du type de données, et non de l'identifiant de terme local qui le nomme. |

| Nœud vierge | L'égalité est limitée à la portée du nœud vierge plus l'étiquette non vide. Les nœuds vierges provenant de différents segments ou de fichiers GTS imbriqués ne sont jamais égaux. Un nœud vierge avec un `"v"` absent ou vide est anonyme : chaque entrée de terme est un nœud vierge distinct dans sa portée. |

| Terme de triplet cité | Égalité des valeurs de termes résolues du sujet, du prédicat et de l'objet du triplet cité. La citation seule n'affirme pas le triplet (§7.3). |

| Nom de graphe | Égalité de la valeur du terme de nom de graphe. Un emplacement de graphe absent est le graphe par défaut et n'est égal à aucun graphe nommé. |

| Blob | Égalité de l'empreinte BLAKE3 normalisée (`blake3:<hex>` ou octets d'empreinte bruts normalisés sous cette forme) ; l'égalité des octets en ligne est prouvée par l'empreinte. |

| Nœud opaque | L'égalité d'une occurrence opaque est son identité de segment plus l'identifiant de contenu de trame. Les présentations exactement identiques PEUVENT (MAY) être réduites par une couche d'affichage, mais le repli préserve l'ordre des occurrences. |

| Métadonnées | Les clés de carte sont comparées par chaîne exacte ; les valeurs sont comparées par l'égalité déterministe du modèle de données CBOR. La vue au niveau du fichier est une fusion superficielle où le dernier l'emporte, tandis que les originaux par segment restent adressables (§7.5). |

| Suppression | Une cible de suppression se résout d'abord dans son propre segment, puis s'applique selon la valeur à l'union des fichiers (§11). Les directives répétées ont un effet d'affichage idempotent mais sont conservées en tant qu'état de repli ordonné. |

<a id="71-terms-terms-frame"></a>

### 7.1 Termes (trame `terms`)

Charge utile : un **tableau ordonné** de termes. Les identifiants sont attribués par ordre d'ajout au sein du dictionnaire de segment actuel (ou au sein d'un dictionnaire `snapshot` avant qu'il ne soit déplacé dans le segment englobant).

```cddl
terms-payload = [+ term]
term = {
  "k"   : 0 / 1 / 2 / 3,   ; 0=IRI 1=literal 2=bnode 3=quoted-triple
  ? "v" : tstr,            ; IRI string | literal lexical form | bnode label
  ? "dt": term-id,         ; literal datatype IRI (a term)
  ? "l" : tstr,            ; literal language tag (BCP 47)
  ? "dir": "ltr" / "rtl",  ; RDF 1.2 base direction for language-tagged literals
  ? "rf": term-id,         ; quoted-triple: the reifier (§7.3) whose triple this term denotes
}
```

**Valeurs par défaut du type de données littéral (normatif).** Pour un terme `k:1` (littéral) : si `"l"` (étiquette de langue) et `"dir"` sont présents et que `"dt"` est absent, le type de données est `rdf:dirLangString` ; si `"l"` est présent, `"dir"` est absent et `"dt"` est absent, le type de données est `rdf:langString` ; si `"l"` et `"dt"` sont tous deux absents, le type de données est `xsd:string`. Une valeur `"dir"` DOIT ÊTRE (MUST) `"ltr"` ou `"rtl"` et n'a aucune signification sans `"l"`.

**Étiquettes de nœuds vierges (normatif).** L'étiquette `"v"` non vide d'un terme `k:2` (nœud vierge) est locale à la portée actuelle du nœud vierge : le segment pour les trames ordinaires, le dictionnaire d'instantané pour un `snapshot`, ou le fichier GTS imbriqué pour la composition récursive (§12.1). Elle NE DOIT PAS (MUST NOT) être traitée comme un identifiant stable à l'échelle mondiale et NE DOIT PAS (MUST NOT) être fusionnée avec la même étiquette dans un autre segment ou GTS imbriqué. Si `"v"` est absent ou correspond à la chaîne vide, le terme est anonyme et dénote un nouveau nœud vierge pour cette entrée de terme au sein de la portée. Les transformations PEUVENT (MAY) réétiqueter les nœuds vierges tout en préservant l'isomorphisme des nœuds vierges et la séparation des portées.

<a id="72-term-id-assignment-normative"></a>

### 7.2 Attribution des term-id (normative)

Les term-id sont des entiers non signés attribués **selon l'ordre d'ajout, par segment**, commençant à `0` à l'en-tête de chaque segment, et sont **figés au sein de leur segment** : un terme créé lors du repli de la trame *N* conserve son identifiant pour le reste de ce segment. Une trame `quads`, `annot` ou `reifies` à la position *N* DOIT (MUST) uniquement référencer des term-id introduits aux positions `0..N-1` **du même segment** (ces trames n'introduisent aucun terme qui leur est propre). Cela rend l'écriture en mode ajout pur, la lecture en une seule passe et la concaténation cohérentes : les term-id sont des **artefacts de compression, jamais d'identité** — l'identité inter-segment est uniquement la *valeur* du terme (§3.1), exactement comme le dictionnaire d'un `snapshot` redémarre déjà à `0` (§10). Une implémentation qui appliquerait des identifiants globaux au fichier à un fichier multi-segment effectuerait un repli incorrect de manière silencieuse ; la règle de délimitation (§3.1) et le vecteur 17 (§19) existent pour rendre cet échec manifeste à la place.

<a id="73-quoted-triples-and-reifiers-reifies-frame"></a>

### 7.3 Triplets cités et réificateurs (trame `reifies`)

RDF 1.2 permet qu'un triplet soit le sujet ou l'objet d'un autre. GTS conserve les triplets cités dans le domaine id : un **réificateur** est un terme IRI/bnode ordinaire ; une trame `reifies` le lie au triplet qu'il cite.

```cddl
reifies-payload = [+ [term-id, term-id, term-id, term-id, ? term-id]] ; reifier, s, p, o, (g)
```

Un triplet cité utilisé comme nœud est un terme avec `"k": 3` et `"rf"` pointant vers son réificateur.

**Mappage d'ensemble de données RDF (normatif).** Un graphe GTS replié correspond à un ensemble de données RDF 1.2 comme suit : chaque rangée `quads` `(S,P,O,G?)` asserte le triplet RDF `(S,P,O)` dans le graphe par défaut lorsque `G` est absent, ou dans le graphe nommé `G` lorsque `G` est présent. Une rangée `reifies` `(R,S,P,O,G?)` asserte le triplet `R rdf:reifies <<( S P O )>>` dans le graphe par défaut lorsque `G` est absent, ou dans le graphe nommé `G` lorsque `G` est présent. Un terme `k:3` dénote ce terme triplet, atteint via son réificateur `R`. Chaque rangée `annot` `(R, P', V', G?)` asserte le triplet `R P' V'` dans le graphe par défaut lorsque `G` est absent, ou dans le graphe nommé `G` lorsque `G` est présent. Les profils PEUVENT (MAY) définir des conventions de placement de graphe supplémentaires pour la projection, mais le mappage de base ci-dessus est la base d'interopérabilité.

**La citation n'implique pas l'assertion (normatif).** Référencer un terme triplet, soit via un réificateur, soit via un terme `k:3`, N'asserte PAS le triplet de base `(S P O)`. Le triplet de base est asserté si et seulement si il apparaît également dans une trame `quads`.

**Dégradation RDF 1.1 (informatif).** RDF 1.1 n'a pas de terme de triplet cité. Une projection RDF 1.1 avec perte PEUT (MAY) remplacer un terme de triplet cité par sa ressource réificatrice et émettre des triplets ordinaires de style réification tels que `R rdf:subject S`, `R rdf:predicate P` et `R rdf:object O`, ou transporter `R rdf:reifies` comme un prédicat d'extension compris par le consommateur. Une telle projection NE DOIT PAS (MUST NOT) asserter `(S P O)` simplement parce que le fichier GTS l'a cité, et l'outillage DEVRAIT (SHOULD) étiqueter la projection comme étant avec perte chaque fois qu'un terme triplet était présent.

<a id="74-quads-and-annotations"></a>

### 7.4 Quads et annotations

```cddl
quads-payload = [+ [term-id, term-id, term-id, ? term-id]]  ; s, p, o, (g; default graph if absent)
annot-payload = [+ [term-id, term-id, term-id, ? term-id]]  ; reifier, predicate, value, (g)
```

Les métadonnées au niveau de l'énoncé (confiance, intervalle de validité, point de vue/perspective, modalité, …) sont exprimées sous forme de lignes `annot` sur un réificateur. **Les affirmations contestées coexistent** : plusieurs lignes `annot` sur un réificateur, ou plusieurs réificateurs sur un (s,p,o), sont tous conservés — aucun n'est privilégié. Les annotations sont un multiensemble ordonné dans l'état de repli GTS, partitionné par le terme de graphe optionnel exactement comme les `quads` : les lecteurs DOIVENT (MUST) préserver l'ordre des lignes au sein de chaque segment et concaténer les lignes d'annotation des segments dans l'ordre du fichier. Les lignes d'annotation identiques exactes sont conservées dans le repli GTS ; une projection de jeu de données RDF PEUT (MAY) fusionner les triplets RDF émis identiques car les jeux de données RDF sont des ensembles.

**Contraintes de position (normatif).** Dans une ligne `quads`, le prédicat `p` DOIT (MUST) être un IRI (`k:0`) ; le sujet `s` DOIT (MUST) être un IRI, un nœud vierge ou un triplet cité (`k:0|2|3`) ; l'objet `o` PEUT (MAY) être n'importe quel terme ; et le nom de graphe `g`, lorsqu'il est présent, DOIT (MUST) être un IRI ou un nœud vierge (`k:0|2`) — jamais un littéral ou un triplet cité. Un triplet `reifies` `(S,P,O)` obéit aux mêmes contraintes sujet/prédicat/objet, et le nom de graphe `g` d'une ligne `reifies` ou `annot`, lorsqu'il est présent, obéit à la même contrainte de nom de graphe que les `quads`. Dans une ligne `annot`, le prédicat DOIT (MUST) être un IRI.

<a id="75-fold-algorithm-normative"></a>

### 7.5 Algorithme de repli (normatif)

```text
result := empty file state
          (terms, quads, reifiers, annotations, blobs, blob_meta, meta,
           suppressions, opaque, signatures, diagnostics, segment ledger)
for segment in file order:                      # §3.1; single-segment files: one iteration
  verify each frame's id (self-hash) and prev-link within the segment;
  record sig status if "sig" present
  terms := []   graph := {}   reif := {}   annot := []
  blobs := {}   blob_meta := {}   meta := {}   suppressed := []   opaque := []
  diagnostics := []
  for frame in segment log order:
    P := resolve payload (§6.1); if undecodable -> add opaque node (§7.6); continue
    switch frame.t:
      "terms"    : append each term (assign next id); each "dt"/"rf" MUST name an
                   already-introduced term-id (no forward references)
      "quads"    : add each (s,p,o,g) value tuple to graph
      "reifies"  : append each (reifier,s,p,o,g) row; a reifier keeps one non-conflicting (s,p,o) binding across graphs (§7.8)
      "annot"    : append (reifier, predicate, value, graph)
      "blob"     : if "d" present -> blobs[BLAKE3(decoded "d")] := bytes (inline);
                   else -> register external blob by "pub".digest;
                   shallow-merge "pub" into blob_meta[digest]
      "suppress" : append directive to `suppressed` (display contract; §11)
      "snapshot" : load a self-contained fold wholesale (§10)
      "meta"     : shallow-merge map into segment meta (later keys overwrite earlier)
      "opaque"   : add explicit opaque node
  union segment fold into result BY TERM VALUE     # ids resolve locally, never cross segments;
                                                   # bnodes keep their scope (§3.1, §12.1)
result
```

Le repli est déterministe : le même journal intact produit le même état de valeur dans chaque lecteur conforme.
Au sein d'un segment, `meta` s'accumule sous la forme d'une union superficielle sur une carte — les clés d'une trame ultérieure remplacent les précédentes ; les valeurs ne sont pas concaténées. **À travers les segments**, le `meta` replié de chaque segment est exposé par segment (indexé par l'identifiant de tête du segment) ET fusionné de manière superficielle dans l'ordre du fichier dans une vue au niveau du fichier — les clés d'un segment ultérieur l'emportent, mais les originaux par segment restent adressables (les métadonnées d'un segment tiers ne sont jamais absorbées silencieusement).

<a id="76-opaque-nodes"></a>

### 7.6 Noeuds opaques

Lorsqu'une charge utile d'une trame ne peut pas être décodée — un codec inconnu ou un codec `cose-encrypt` pour lequel le lecteur ne possède aucune clé — le lecteur NE DOIT PAS (MUST NOT) la supprimer. Il DOIT (MUST) ajouter un **noeud opaque** au graphe transportant tout ce qui demeure en texte clair :

```cddl
opaque-node = {
  "id"      : content-id,      ; the frame's self-hash
  "type"    : frame-type,      ; declared "t"
  ? "pub"   : any,             ; the cleartext public envelope, if any
  ? "to"    : [+ recipient],   ; declared recipients
  "sigstat" : "none" / "valid" / "invalid" / "unverified",
  "reason"  : "unknown-codec" / "missing-key" / "damaged",
}
```

La plupart des noeuds opaques sont produits par un lecteur au moment du décodage ; un rédacteur PEUT (MAY) également émettre une trame `opaque` explicite (par ex. un substitut de caviardage) dont la charge utile est la structure ci-dessus, auquel cas `"sigstat"` est omis (un lecteur le détermine). Une trame `damaged` (auto-hachage échoué ou absent) est isolée et repliée également comme un noeud opaque (§9.1) : un lecteur PEUT (MAY) exposer ses champs en texte clair comme métadonnées de diagnostic **non fiables** (untrusted), mais DOIT (MUST) définir `"sigstat"` à `invalid`/`unverified` et `"reason": "damaged"` — les octets ne sont pas dignes de confiance. La trame participe toujours à la chaîne id/prev, elle ne peut donc pas être supprimée silencieusement.

<a id="77-streaming-fold-and-bounded-memory"></a>

### 7.7 Repli diffusable en continu (streaming) et mémoire limitée

Un graphe n'a pas besoin d'être matérialisé pour être *transformé*. Un **lecteur diffusable en continu (Streaming Reader)** (§2.1) traite les trames dans l'ordre et les émet vers un récepteur (sink), ne conservant que le dictionnaire de termes, la trame ou le blob décodé actuel, ainsi que l'état id/prev et de validation en cours :

- `gts → duckdb`/`sqlite` (§14) conservent le modèle d'**identifiants entiers (integer-id)** : diffusent les deltas `terms` dans une table `terms` et les deltas `quads`/`reifies`/`annot` dans des tables de valeurs d'identifiants, en effectuant des insertions massives (bulk-insert) à mesure que les trames arrivent. **Aucune résolution de termes ni matérialisation de graphe ne se produit** — la mémoire est limitée par le dictionnaire, la plus grande trame décodée et l'état du sidecar de validation plutôt que par les triplets ou les blobs. La jointure relationnelle qui résout les identifiants est la tâche du moteur, ultérieurement.

- `gts → ttl/nq` doit résoudre les identifiants pour émettre du texte. Si le dictionnaire dépasse la mémoire, le lecteur utilise le localisateur d'index `"dict"` (§6.2) pour charger (ou mapper en mémoire, ou déverser dans un magasin clé-valeur sur disque) uniquement les trames `terms` d'abord, puis diffuse les quads.

Même O(termes distincts + taille maximale de la trame décodée + état du sidecar de validation) peut dépasser la mémoire pour des graphes pathologiquement irréguliers (par exemple, une exploration Web déversant des millions d'IRI UUID uniques, ou un seul blob en ligne très volumineux). Un lecteur diffusable en continu PEUT (MAY) donc **vider (flush) son dictionnaire en mémoire vers un magasin clé-valeur temporaire sur disque** lorsqu'une limite de mémoire est atteinte, échangeant de la RAM contre un fichier de déversement (spill) local ; l'exactitude n'est pas affectée car les term-ids sont ordonnés par ajout (append-order) et figés (§7.2). Les transformations `gts → duckdb`/`sqlite` en bénéficient gratuitement — la table cible *est* le déversement.

La limite de revendication du récepteur diffusable (streaming-sink) au niveau du paquet et l'assistant de test de performance (benchmark) de la mémoire sont maintenus dans [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md).

Un journal de plusieurs gigaoctets se transforme ainsi en un substrat d'exploitation en mémoire limitée — le mode d'échec OOM (épuisement de la mémoire) par résolution et matérialisation est évité par construction.

<a id="78-duplicates-and-conflicts-normative"></a>

### 7.8 Doublons et conflits (normatif)

Tous les comportements relatifs aux doublons et aux conflits sont définis ici afin que les gestionnaires de trames n'inventent pas de politique locale :

| élément | comportement de repli |

|---|---|

| Termes en double | Un rédacteur DEVRAIT (SHOULD) interner les termes répétés, mais chaque entrée de terme reçoit toujours son propre identifiant local au segment. Les valeurs non vierges qui sont égales selon le §7 sont la même valeur dans l'union de fichiers. Les nœuds vierges anonymes (`"v"` absent ou vide) sont nouveaux pour chaque entrée de terme. |

| Quads en double | Le graphe replié est un ensemble : les rangées de valeurs `(s,p,o,g)` identiques fusionnent en une seule sans diagnostic. |

| Rangées de réificateur | Un réificateur DEVRAIT (SHOULD) être lié à exactement une identité de triplet `(s,p,o)`. Les rangées `(reifier,s,p,o,g)` identiques répétées sont sans conséquence. Le même réificateur PEUT (MAY) apparaître dans plusieurs graphes seulement si `(s,p,o)` ne change pas. Un `(s,p,o)` conflictuel pour le même réificateur est une erreur de qualité des données : le lecteur affiche `ConflictingReifier`, conserve la première identité de triplet dans l'ordre du fichier et ignore les rangées de réificateur conflictuelles. |

| Annotations | Les rangées d'annotations sont un multi-ensemble ordonné (§7.4). Plusieurs rangées sur un même réificateur coexistent, y compris les rangées partitionnées par graphe ; les rangées exactement identiques sont conservées dans le repli GTS. Les projections de jeux de données RDF peuvent fusionner les triplets émis identiques. |

| Octets de blob | Les blobs sont adressés par condensé. La répétition du même condensé/octets est idempotente ; une vue adressée par le contenu stocke une seule valeur d'octet par condensé. L'extraction de validation recalcule le hachage des octets intégrés par rapport au condensé demandé (§14.1). |

| Métadonnées de blob | `blob_meta[digest]` est une carte superficielle construite dans l'ordre du fichier. Les clés de métadonnées ultérieures pour le même condensé remplacent les clés antérieures dans la vue au niveau du fichier ; les déclarations antérieures restent dans les trames originales. |

| Suppressions | Les directives de suppression sont additives. La répétition d'une directive équivalente a un effet d'affichage idempotent, mais elle reste présente dans la liste de suppression ordonnée. Il n'y a pas de trame de désuppression (§11). |

| Clés de métadonnées | Le `meta` de segment est superficiel selon le principe « le dernier l'emporte » ; le `meta` de fichier est la fusion superficielle « le dernier l'emporte » des métadonnées de segment dans l'ordre du fichier. Les métadonnées par segment restent adressables (§7.5). |

| Trames malformées | Une trame dont la charge utile ne peut être décodée ou dont le gestionnaire ne peut la replier en toute sécurité devient un nœud opaque avec un diagnostic lorsque la récupération est possible (§7.6, §9.1). Les trames ultérieures survivantes sont toujours repliables lorsque les limites des éléments sont connues. |

| Types de trames structurelles inconnus | Un lecteur de base (Baseline Reader) n'attribue pas de sémantique de graphe à un type de trame inconnu. Il DOIT (MUST) préserver la vérification de la chaîne et PEUT (MAY) afficher un nœud opaque ou un diagnostic ; un lecteur complet (Full Reader) sensible aux profils PEUT (MAY) interpréter la trame. |

| Conflits de profil | L'union des déclarations de profil et des exigences de profil s'effectue à travers les segments (§3.1, §13). Les exigences de profil non prises en charge dégradent les charges utiles non prises en charge du segment concerné en nœuds opaques ou en diagnostics de profil ; elles n'invalident pas par elles-mêmes le repli du format de transmission (wire-format) de base. |

<a id="8-transform-catalog"></a>

## 8. Catalogue des transformations

<a id="81-classes"></a>

### 8.1 Classes

Chaque entrée de catalogue déclare une **classe** :

| classe      | exemples                         | capacité requise pour inverser |

|------------|----------------------------------|------------------------------|

| `encode`   | `identity`, `base64`, `base85`   | aucune (fonction pure)         |

| `compress` | `gzip`, `zstd`, `lzma2`          | une bibliothèque de codec              |

| `encrypt`  | `cose-encrypt0`, `cose-encrypt`  | une **clé** (par destinataire)    |

<a id="82-stacking"></a>

### 8.2 Empilement

`"x"` est appliqué dans l'ordre du tableau lors de l'encodage et inversé lors du décodage. Exemple : `[zstd,
cose-encrypt]` signifie *compresser, puis chiffrer* ; un lecteur déchiffre (si muni d'une clé) puis décompresse.

<a id="83-capability-model-and-graceful-degradation"></a>

### 8.3 Modèle de capacité et dégradation gracieuse

Le décodage d'une chaîne requiert **chaque** capacité qu'elle nomme. Une capacité manquante est traitée uniformément, qu'il s'agisse d'une bibliothèque (`unknown-codec`) ou d'une clé (`missing-key`) : la trame devient un noeud opaque (§7.6). Ce mécanisme unique permet la **négociation de contenu intégrée au fichier** — un objet logique PEUT (MAY) apparaître sous forme de plusieurs trames dans différents codecs/formats (p. ex. une représentation haute fidélité qu'un lecteur ne peut pas décoder *et* un substitut largement pris en charge qu'il peut décoder), et le lecteur utilise la meilleure trame pour laquelle il possède les capacités.

<a id="84-mandatory-core-set-and-durability"></a>

### 8.4 Ensemble central obligatoire et durabilité

Un lecteur (Reader) de base DOIT (MUST) implémenter `identity`, `gzip` et `zstd` — ainsi, le jeu de dépendances complet d'un lecteur conforme est **CBOR + BLAKE3 + gzip + zstd**. Les rédacteurs (Writers) visant une longévité maximale DEVRAIENT (SHOULD) se restreindre à l'ensemble central. Les rédacteurs (Writers) axés sur la densité PEUVENT (MAY) utiliser `lzma2` avec un dictionnaire en bande. Tous les codecs centraux sont des primitives stables et largement déployées.

**Codecs rsyncables.** Un codec de classe `compress` PEUT (MAY) être *rsyncable* : il synchronise (réinitialise) périodiquement son état de compression de sorte qu'un changement local dans l'entrée non compressée n'affecte qu'un voisinage limité de la sortie compressée. Cela améliore les outils de transfert delta (p. ex. `rsync`) et la compression delta du contrôle de version (p. ex. les fichiers packs Git) au prix d'un léger surcoût du taux de compression. Le seul codec rsyncable défini dans cette révision est `zstd-rsyncable` (§8.5).

<a id="85-canonical-codec-registry-v1"></a>

### 8.5 Registre canonique des codecs (v1)

Les entrées du catalogue sont référencées par un identifiant entier au sein d'un fichier (§5), mais le `"name"` de chaque entrée DOIT (MUST) être un identifiant canonique de ce registre afin que les rédacteurs (writers) interopèrent :

| nom            | classe        | de base ? | paramètres                    |

|-----------------|------------|-----------|-------------------------------|

| `identity`      | `encode`   | oui       | aucun                          |

| `gzip`          | `compress` | oui       | `level`?                      |

| `zstd`          | `compress` | oui       | `level`?, `window`?, `dct`?   |

| `zstd-rsyncable`| `compress` | non        | `block_size` : uint (par défaut 65536) |

| `lzma2`         | `compress` | non        | `level`?, `dct`?              |

| `base64url`     | `encode`   | non        | aucun (sans remplissage)               |

| `base85`        | `encode`   | non        | aucun                          |

| `cose-encrypt0` | `encrypt`  | non        | `COSE_Encrypt0` (1 destinataire) |

| `cose-encrypt`  | `encrypt`  | non        | `COSE_Encrypt` (n destinataires) |

Un lecteur (reader) DOIT (MUST) faire correspondre les codecs par leur `"name"` canonique, et non par l'identifiant du catalogue (les identifiants sont locaux au fichier). Les versions ultérieures de la spécification enregistrent de nouveaux codecs par leur nom canonique ; un nom inconnu se dégrade en un noeud opaque (opaque node) (§8.3).

<a id="9-integrity-and-confidentiality"></a>

## 9. Intégrité et confidentialité

GTS distingue quatre préoccupations d'intégrité :

1. **Intégrité des trames** — l'auto-hachage BLAKE3 par trame `"id"` (§9.1).

2. **Intégrité de l'historique** — la chaîne d'identifiants de contenu `"prev"` (§9.1).

3. **Origine / paternité** — signatures COSE facultatives (§9.2).

4. **Fraîcheur / non-troncation** — un engagement de tête : une signature sur la tête `"id"`, ou une racine d'index `"mmr"`/`"head"` (§9.1, §13).

Les deux premières sont obligatoires et sans clé ; les deux dernières sont facultatives et définies par le profil.

<a id="91-per-frame-self-hash-and-content-id-chain-mandatory"></a>

### 9.1 Auto-hachage par trame et chaîne d'identifiants de contenu (obligatoire)

Le `"id"` de chaque trame est le BLAKE3-256 de son propre contenu (chaque clé sauf `"id"` et `"sig"`), de sorte qu'une trame est **adressée par son contenu et vérifiable de manière indépendante**. Le `"prev"` de chaque trame nomme le `"id"` de la trame précédente ; puisque `"prev"` fait partie du contenu haché, la chaîne est une **liste adressée par contenu de style git dans laquelle l'identifiant de tête s'engage de manière transitive envers tout l'historique**.

- **Vérification parallèle.** Chaque `"id"` est un hachage d'une plage d'octets autonome ; avec la table d'index `"off"` (§6.2), tous les hachages de trame sont re-calculés simultanément, suivis d'une passe d'égalité `"prev"` triviale en O(n). Aucune dépendance accumulée n'impose une lecture monothread. (La seule étape intrinsèquement séquentielle est la découverte des limites de trame dans une séquence CBOR brute — un balayage de longueur peu coûteux que l'index élimine.)

- **Isolation des dommages et récupération.** Une trame corrompue échoue à son propre `"id"`, de sorte que le dommage est **détectable de manière indépendante**. La récupération des trames *subséquentes*, cependant, n'est garantie que lorsque leurs décalages d'octets (offsets) sont connus — à partir d'une table `index` `"off"` intacte, d'une trame de point de contrôle, d'un tramage externe ou de la couche de stockage. Dans une séquence CBOR brute (sans longueur par trame), une corruption d'octets arbitraire peut désynchroniser le décodeur : un lecteur **avec** des décalages saute la mauvaise trame et replie les survivantes (`reason: "damaged"`), tandis qu'un lecteur **sans** décalages PEUT (MAY) être incapable de se resynchroniser au-delà du dommage. Les rédacteurs `evidence` DEVRAIENT (SHOULD) émettre des index de points de contrôle périodiques (§13) afin que la récupération soit robuste.

- **Preuve d'altération.** Toute insertion, réorganisation ou mutation rompt un lien `"prev"` ou un auto-hachage. **La troncature** (suppression de trames de fin) n'est détectée que par rapport à un engagement de tête — une signature sur le `"id"` de tête, la racine `"head"`/`"mmr"` de l'index (§6.2), ou une ancre hors bande. Les trames opaques font partie de la chaîne, de sorte que les trames confidentielles ne peuvent pas être retirées de manière indécelable.

Une racine **Merkle-Mountain-Range** (MMR) sur les identifiants de trame (optionnelle, transportée dans l'index) est un engagement unique pour l'ensemble du fichier qui est lui-même parallèle à calculer et prend en charge les preuves d'inclusion en O(log n) — prouvant qu'une trame se trouve dans le journal sans expédier le journal.

<a id="92-signatures-optional-algorithm-agile"></a>

### 9.2 Signatures (facultatif, agilité algorithmique)

Une trame PEUT (MAY) porter `"sig"`, une `COSE_Sign1` (RFC 9052) sur le `"id"` de la trame. Puisque `"id"` est l'auto-hachage de l'ensemble du contenu — `"pub"`, `"d"` (le texte chiffré, s'il est chiffré) et `"prev"` (la position dans la chaîne) — une signature sur `"id"` **lie** les revendications publiques à la charge utile scellée et à la position dans la chaîne, et signer la tête `"id"` ancre ainsi tout l'historique antérieur (§9.1). L'algorithme de signature est déclaré dans l'en-tête COSE (par ex. `EdDSA`/Ed25519, `ES256`) ; les lecteurs DOIVENT (MUST) honorer l'algorithme déclaré. Les profils `evidence` et `opaque` (§13) EXIGENT des signatures. La découverte de clés et l'ancrage de la confiance (quelles clés sont authentiques, quels signataires sont autorisés) relèvent de la **politique de profil/déploiement**, et non du cœur de GTS : `sigstat: "valid"` signifie qu'une signature est cryptographiquement valide sous une clé *résolue*, et non que la clé est de confiance.

<a id="93-encryption-optional"></a>

### 9.3 Chiffrement (facultatif)

Un codec de classe `encrypt` enveloppe la charge utile sous forme de `COSE_Encrypt`/`COSE_Encrypt0`. Les destinataires sont répertoriés en texte clair dans `"to"` par **identifiant de clé uniquement** — jamais par le matériel de clé. Plusieurs destinataires PEUVENT (MAY) partager une seule charge utile scellée (chacun déballe la clé de chiffrement de contenu avec sa propre clé). Le séquestre, la rotation et la révocation des clés relèvent de la responsabilité de l'**émetteur** et sont hors de portée ; une charge utile chiffrée pour une clé retirée PEUT (MAY) devenir définitivement opaque.

La surface de conformité v1 de cette ébauche implémente et teste `COSE_Encrypt0` pour un destinataire direct. Les enveloppes `COSE_Encrypt` multi-destinataires et l'enveloppement de clé ECDH sont reportés au-delà de la v1 jusqu'à ce que des vecteurs dédiés, une politique de gestion des clés et des tests d'interopérabilité inter-moteurs existent ; voir [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md).

Le contrat `cose-encrypt` différé est épinglé comme une future capacité de Lecteur Complet (Full Reader), et non comme une revendication d'implémentation v1. Une implémentation future qui le revendique DOIT (MUST) consommer une enveloppe `COSE_Encrypt` étiquetée CBOR dont le chiffrement de contenu utilise `A256GCM` et dont le tableau de destinataires contient une entrée par destinataire déclaré dans la liste `"to"` en texte clair de la trame. Le mode de gestion des clés initialement défini est `ECDH-ES+A256KW` : chaque entrée de destinataire porte les informations nécessaires pour dériver une clé de chiffrement de clé et déballer la même clé de chiffrement de contenu de 256 bits avec `A256KW`. Un lecteur futur conforme n'essaie que les destinataires pour lesquels il détient le matériel de clé privée correspondant.

Les modes de défaillance différés sont :

- aucune clé détenue ne correspond à un destinataire `kid` : émettre `MissingKey` et conserver la trame en tant que nœud opaque avec `reason:"missing-key"` ;

- les métadonnées de destinataire ECDH sont malformées, la clé de chiffrement de clé dérivée ne peut pas déballer la clé de contenu, ou l'authentification AES-KW échoue : émettre `KeyWrapFailed` et conserver la trame comme opaque avec `reason:"missing-key"` ;

- une défaillance d'authentification de contenu après un déballage réussi constitue une charge utile chiffrée endommagée et NE DOIT PAS (MUST NOT) exposer le texte en clair.

Les vecteurs descripteurs dans `vectors/crypto-deferred/*.json` épinglent la forme à deux destinataires et les cas d'opacité de clé erronée, de clé manquante et d'échec de déballage jusqu'à ce que des vecteurs `COSE_Encrypt` au niveau de l'octet remplacent les espaces réservés.

<a id="94-the-opacity-invariant-normative"></a>

### 9.4 L'invariant d'opacité (normatif)

> L'opacité masque le **contenu** — jamais l'**existence**, la **provenance** ou la **position**.

Pour chaque trame, `{"id", "prev", "t", "x", "to", "pub", "sig"}` DOIT (MUST) rester en texte clair (la chaîne de transformation `"x"` est en texte clair pour qu'un lecteur sache quels codecs inverser). Un lecteur sans la clé pertinente apprend donc tout de même *que* la trame existe, *quel type* elle est, pour *qui* elle est scellée, *qui* l'a signée et *où* elle se situe dans la chaîne. C'est ce qui rend la divulgation sélective sûre : un détenteur peut transporter — et un vérificateur peut authentifier la position de — des données qu'aucun des deux ne peut lire.

<a id="10-compaction"></a>

## 10. Compaction

La compaction replie un journal et le réémet sous la forme d'une trame `snapshot` unique et
autonome (dictionnaire ré-interné, quads dédoublonnés, boucles réflexives supprimées,
optionnellement une fermeture d'inférence matérialisée). La charge utile d'un `snapshot`
est un repli de graphe autonome — termes, quads, réificateurs, annotations, blobs en ligne et méta :

```cddl
snapshot-payload = {
  "terms"    : terms-payload,
  ? "quads"  : quads-payload,
  ? "reifies": reifies-payload,
  ? "annot"  : annot-payload,
  ? "blobs"  : { * digest => bstr },   ; inline content-addressed blobs
  ? "meta"   : any,
}
```

Un lecteur replie un `snapshot` exactement comme il replierait la séquence équivalente de
trames `terms`/`quads`/
`reifies`/`annot`/`blob` ; les identifiants de termes redémarrent à `0` au sein du
dictionnaire propre de l'instantané.
La compaction est **avec perte par définition** : elle rejette les signatures d'origine par trame et
l'empilement temporel du journal. Un compacteur :

- DOIT (MUST) enregistrer la provenance du repli (condensé du journal source, heure, agent) comme quads dans
  l'instantané, et

- DEVRAIT (SHOULD) émettre une nouvelle signature sur l'instantané.

Deux classes d'artefacts en découlent : un **journal probatoire** (à ajout exclusif, signé, jamais compacté) et
un **instantané de distribution** (compacté, dense, avec perte — idéal pour l'expédition). Un lecteur peut savoir
lequel il détient à partir du profil et de la présence d'une trame `snapshot`.

<a id="101-streamable-compaction-ordering-only"></a>

### 10.1 Compaction diffusable en continu (ordonnancement uniquement)

La compaction diffusable en continu convertit un segment accrétif (ou un fichier multi-segment) en un segment ordonné pour la livraison dans l'état de la disposition diffusable en continu (§3.3). Contrairement à la compaction par instantané (snapshot) ci-dessus, il s'agit d'une **réécriture de l'ORDONNANCEMENT, et seulement de l'ordonnancement** : le graphe replié (folded graph), les blobs en ligne et chaque fait adressé par le contenu sont préservés. Trois sujets de signature se comportent différemment sous la réécriture, et un compacteur DOIT (MUST) respecter les trois :

- **Signatures de contenu** (sujet = un condensé (digest) de contenu : le BLAKE3 d'un blob, un hachage d'énoncé ou de revendication — « ceci est vrai, signé par Bob ») sont des quads/annotations ordinaires sur les condensés. Elles sont **invariantes à la compaction** et survivent intactes : rien de ce qu'elles attestent n'a changé.

- **Signatures de trame** (un COSE_Sign1 sur une trame `"id"`, qui s'engage envers `"prev"`, §9.2) deviennent **détachées, et non brisées** : elles se vérifient par rapport à l'identifiant de trame original à jamais. Un compacteur DOIT (MUST) transporter chaque signature de trame source dans la **provenance de compaction** — un nœud `stream:DetachedSignature` par signature, enregistrant l'identifiant de trame original (`stream:sourceFrame`) et les octets COSE originaux (`stream:cose`), plus un `stream:sourceHead` par tête de segment source (§13.3) — afin que chacune demeure une *revendication vérifiable sur le journal (log) original*.

- **Engagements d'ordonnancement** (une tête signée, une racine d'index `"mmr"`) sont les seules attestations liées à la disposition. Ils ne peuvent pas survivre à un réordonnancement ; le compacteur réémet l'engagement d'ordonnancement (le nouveau `index` de fin avec son `"head"`, §6.2) et devient ainsi le **seul attesteur du nouvel ordonnancement**. Un compacteur PEUT (MAY) également signer par COSE la nouvelle tête.

Un compacteur DOIT (MUST) enregistrer la réécriture elle-même sous forme de quads de provenance dans la sortie — un nœud `stream:Compaction` portant l'outil agissant (`stream:agent`), l'heure (`stream:timestamp`) et les têtes de segments sources (`stream:sourceHead`) — la provenance du §10 DOIT (MUST), compte tenu du vocabulaire concret du §13.3.

**Profils exigeant une attestation de chaîne tierce vierge.** Pour un segment `evidence`, la chaîne signée originale *est* l'artefact ; un compacteur DOIT (MUST) le refuser — à moins qu'il ne **scelle le journal original textuellement (verbatim)** en tant que blob GTS imbriqué (§12.1) à l'intérieur de la réécriture diffusable en continu (rôle `"source"`, référencé depuis le nœud de provenance via `stream:sealedSource`). Les octets, la chaîne et les signatures originaux restent intacts au niveau de l'octet et vérifiables de manière indépendante à l'intérieur ; la disposition externe est ordonnée pour la livraison ; un condensé de contenu les lie.

**Refus pour les outils de publication (§14.1).** Un compacteur DOIT (MUST) refuser : une entrée qui ne se vérifie pas proprement (tout diagnostic) ; et une entrée dont le repli (fold) porte une suppression adressée par trame (`kind: "frame"`, §11) — la réécriture attribue de nouveaux identifiants de trame, de sorte qu'une cible de condensé de trame resterait silencieusement orpheline. Les suppressions `blob` adressées par condensé sont reportées textuellement (l'adressage par contenu est indépendant de la disposition) ; les suppressions adressées par identifiant sont reportées par valeur (§11).

<a id="11-suppression-additive-deletion"></a>

## 11. Suppression (« suppression » additive)

GTS ne supprime jamais physiquement. Pour rétracter ou masquer du contenu antérieur, un rédacteur (writer) ajoute une trame `suppress` référençant le condensé (digest) du sous-graphe ou de la trame supplanté. Les octets supprimés demeurent présents et liés par hachage ; la suppression est un **contrat d'affichage/de préséance**, interprété par le consommateur, et non un effacement. Cela préserve un historique complet et infalsifiable.

```cddl
suppress-payload = { "targets": [+ suppress-target], ? "reason": tstr, ? "by": term-id }
suppress-target =
    { "kind": "frame",   "id": digest } /                                ; a frame, by its "id"
    { "kind": "blob",    "digest": digest } /                            ; a content-addressed blob
    { "kind": "term",    "id": term-id } /                               ; a term + quads it appears in
    { "kind": "quad",    "q": [term-id, term-id, term-id, ? term-id] } / ; one specific quad
    { "kind": "reifier", "id": term-id }                                 ; a reifier + its annotations
```

La suppression est **monotone et additive** : une cible correspondante est masquée de la résolution par défaut (une cible `term` masque également chaque quad dans lequel le terme apparaît) ; les octets demeurent présents et liés par hachage, et un consommateur PEUT (MAY) afficher explicitement le contenu supprimé. Il n'y a pas de « désuppression » dans la v1 — des trames ultérieures peuvent ajouter d'autres suppressions, et une assertion identique ultérieure ne rétablit pas une cible supprimée.

**Suppression inter-segment (normative, §3.1).** Les cibles adressées par condensé (`frame`, `blob`) sont globales au fichier : un identifiant de contenu désigne les mêmes octets quel que soit leur emplacement, de sorte qu'un segment ultérieur PEUT (MAY) supprimer par condensé une trame ou un objet binaire (blob) d'un segment antérieur. Les cibles adressées par identifiant (`term`, `quad`, `reifier`) portent des identifiants de termes, qui sont locaux au segment — ils sont d'abord **résolus en valeurs de termes au sein du propre segment de la trame de suppression**, puis la suppression s'applique alors **par valeur à l'ensemble du repli (fold) d'union** : une cible `quad` masque chaque tuple de valeurs `(s,p,o,g)` correspondant dans n'importe quel segment, et une cible `term` masque la valeur du terme (et les quads dans lesquels elle apparaît) à l'échelle du fichier. C'est ce qui permet à un segment de révision de croyances ajouté de supprimer une déclaration faite par un segment antérieur sans en réécrire un seul octet — l'enregistrement du segment antérieur reste présent, signé et lié par hachage (adressé par contenu au niveau filaire).

<a id="12-binary-and-content-addressing"></a>

## 12. Binaire et adressage par contenu

```cddl
; a `blob` frame carries raw bytes in "d" (subject to "x"); its metadata lives in cleartext "pub":
blob-pub = { ? "mt": tstr, ? "rep": tstr, ? "digest": digest-ref }
; INLINE blob  -> "d" present; digest = BLAKE3(decoded "d").
; EXTERNAL blob -> "d" absent;  "pub".digest names bytes held elsewhere.
```

- Les octets d'une trame `blob` sont adressés par leur **condensé BLAKE3-256** — pour un blob en ligne, le `BLAKE3` du `"d"` décodé, pour un blob externe, `"pub".digest` ; le graphe référence le blob par ce condensé. Des octets identiques apparaissant deux fois sont stockés une seule fois par convention.

- Un blob PEUT (MAY) être **en ligne** (octets présents, un paquet autonome) ou **externe** (seul le condensé apparaît dans le graphe ; les octets résident ailleurs).

- Un objet logique PEUT (MAY) avoir de **multiples représentations** (`"rep"`/`"mt"` distinguant, par exemple, une version maîtresse et un substitut de repli largement pris en charge) — voir la négociation de contenu, §8.3.

- La transformation vers un format texte (§14) externalise les blobs en ligne vers un répertoire auxiliaire.

<a id="121-nested-gts-recursive-composition"></a>

### 12.1 GTS imbriqué (composition récursive)

Un blob dont le type de média est `application/vnd.blackcat.gts+cbor-seq` est en soi un fichier GTS complet.
Puisqu'un contenu utile après l'inversion de la transformation est constitué d'octets opaques, toute charge utile de trame PEUT (MAY) transporter un GTS imbriqué, enveloppé dans n'importe quelle chaîne de transformation — `[zstd]`, `[cose-encrypt]`, ou les deux. Le transporteur normatif est un `blob` dont le `"pub".mt` est `application/vnd.blackcat.gts+cbor-seq`.

- **Sémantique de repli.** Un Lecteur Complet PEUT (MAY) effectuer une récursion : décoder le blob (sous réserve des règles de capacité du §6.1), puis replier les octets internes en tant que GTS indépendant, exposant son résultat comme un **sous-graphe** auquel le graphe parent fait référence par le condensé (digest) du blob. Un Lecteur de Base PEUT (MAY) traiter un GTS imbriqué comme un blob ordinaire (pas de récursion).

- **Portée des blank-nodes.** Le GTS interne possède une portée de nœuds vierges (blank-node) indépendante. Si un Lecteur Complet expose le repli interne à côté du repli parent, il DOIT (MUST) réétiqueter ou délimiter la portée des nœuds vierges internes afin que les étiquettes ne puissent pas entrer en collision avec le parent ou avec des fichiers GTS imbriqués frères.

- **Intégrité indépendante.** Le GTS interne possède son propre en-tête, sa propre chaîne id/prev et ses propres signatures. La chaîne **externe** prouve que le blob imbriqué est présent et intact à sa position ; la chaîne **interne** prouve que le journal (log) imbriqué est intact. Les deux garanties se composent mais ne dépendent pas l'une de l'autre.

- **Opacité composée.** Si le GTS imbriqué est atteint via une transformation de classe `encrypt` et que le lecteur ne possède pas la clé, l'*ensemble du sous-graphe* — y compris son en-tête interne — est un nœud opaque (§7.6) : le détenteur peut transporter et prouver la position d'un graphe scellé complet qu'il ne peut pas lire. C'est le cas matryoshka (« un GTS complet à l'intérieur d'un champ chiffré »).

- **Récursion bornée.** Les lecteurs DOIVENT (MUST) appliquer une profondeur d'imbrication maximale et un budget de taille totale décodée (§18).

Cette composition ne nécessite aucun nouveau type de trame : l'imbrication est « un blob qui se trouve être un GTS ».
L'assistant du Lecteur Complet v1 et les vecteurs de sécurité négatifs pour les limites de récursion sont suivis dans [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md).

<a id="13-profiles"></a>

## 13. Profils

Un profil est un ensemble nommé de conventions s'appliquant au format unique, déclaré dans le champ `"prof"` de l'en-tête de segment. Les profils peuvent définir des attentes en matière de vocabulaire, des règles de validation, des politiques de confiance, des exigences de capacité et des flux de publication, mais ils se situent au-dessus du format filaire de base.

Les valeurs de statut utilisées par cette spécification sont :

- **core-required** : fait partie de la conformité de base du format filaire, du lecteur ou du rédacteur.

- **optional-standard** : spécifié ici comme infrastructure interopérable, mais n'est pas requis pour un
  Lecteur ou un Rédacteur de base.

- **experimental** : décrit pour une interopérabilité précoce ; les détails peuvent changer sans modifier
  le cœur de GTS.

- **domain-specific** : appartient à une application ou à une communauté en aval, et non au cœur de GTS.

| profil ou famille | statut | signification au niveau du profil | impact sur le cœur |

|---|---|---|---|

| `generic` | core-required par défaut | Tout journal conforme sans validation de profil supplémentaire. | Aucun ; il s'agit de l'absence d'exigences spécifiques au profil. |

| `dist` | optional-standard | Une distribution compactée `snapshot` : vocabulaire, définitions et clôture matérialisée. | Aucun. |

| `evidence` | optional-standard | Chaîne de garde en ajout uniquement ; les validateurs de profil exigent des signatures et un engagement de tête (head commitment). | Aucun ; les signatures demeurent facultatives dans le cœur de GTS. |

| `opaque` | optional-standard | Convention de divulgation sélective sur les trames de classe `encrypt`, les signatures et les `kid` pseudonymes. | Aucun ; le chiffrement demeure facultatif dans le cœur de GTS. |

| `bundle` | optional-standard | Un GTS dont les `blob` sont eux-mêmes des fichiers GTS (`mt: application/vnd.blackcat.gts+cbor-seq`), utilisant le §12.1. | Aucun. |

| `files` | optional-standard | Un profil d'archive d'arborescence de fichiers portable défini aux §13.2 et §14.2. | Aucun ; les lecteurs de base replient son graphe normalement. |

| `stream` | optional-standard | Support du vocabulaire de diffusion en continu et de la disposition de publication utilisé par les §3.3 et §10.1. | Aucun ; les vérifications de disposition sont des diagnostics du lecteur ou de l'outil, et non une nouvelle grammaire de trame. |

| `image` | experimental | Représentations de blobs plus métadonnées descriptives et trames d'analyse. | Aucun. |

| `ai-package` | experimental | Un concept plus logique, observations, opinions, affirmations réfutées, plongements (embeddings) et données. | Aucun. |

| `music-package` | domain-specific | Conventions de transport de musique GMEOW ; informatif ici, spécifié par le profil en aval. | Aucun. |

| Profils de distribution GMEOW | domain-specific | Conventions de paquet GMEOW en aval superposées aux artefacts de distribution GTS. | Aucun. |

| `agent-memory` | domain-specific | Conventions d'application pour la mémoire, la révision de croyance, la suppression et la provenance. | Aucun. |

Les profils contraignent les conventions, non le format filaire ; un lecteur `generic` lit toutes les déclarations de profil qu'il peut analyser. Un lecteur qui n'implémente pas un profil nommé analyse, vérifie et replie tout de même le fichier sous sa classe de lecteur, puis signale le profil non supporté dans les diagnostics ou les métadonnées. Dans un fichier multi-segment, chaque segment déclare son propre profil ; l'ensemble des exigences effectives du fichier est l'union (§3.1).

Le profil `evidence` exige un engagement de tête (head commitment) (§9, point 4) au niveau du profil, et les rédacteurs DEVRAIENT (SHOULD) émettre un point de contrôle (checkpoint) `index` au moins toutes les 1024 trames ou tous les 64 MiB, selon la première éventualité, afin qu'un journal endommagé se rétablisse de manière robuste (§9.1). Cette exigence ne rend pas obligatoires les signatures, les index ou le support du profil de preuve pour le GTS de base.

**Configuration de la politique de profil.** Un vérificateur sensible aux profils PEUT (MAY) accepter une politique de confiance de déploiement à côté des octets GTS. Le document de politique v1 est en JSON ou YAML avec ces champs :

```yaml
trusted_signers:
  - did:example:issuer
require_trusted_signer: true
pseudonymous_kid_pattern: "^anon:[0-9a-fA-F]{32,}$"
```

`trusted_signers` énumère les valeurs `kid` du signataire autorisées par le déploiement.
`require_trusted_signer` fait échouer les profils qui exigent des signatures, à moins qu'au moins une signature valide ne provienne d'un signataire de confiance. `pseudonymous_kid_pattern` contrôle la forme de l'identifiant de destinataire (recipient-id) à haute confidentialité pour le profil `opaque`. Ces paramètres concernent uniquement la politique de profil ou de déploiement : ils ne modifient pas l'analyse du cœur de GTS, le repli, les identifiants de trame, les préimages de signature ou la validité du lecteur de base.

**Modèle d'enregistrement de profil tiers.** Une définition de profil tiers DEVRAIT (SHOULD) publier :

- Nom de profil stable utilisé dans le champ d'en-tête `"prof"`.

- Propriétaire, processus de contrôle des changements, URI de contact et URI de spécification.

- Statut (`experimental`, `optional-standard` ou `domain-specific`) et politique de compatibilité
  prévue.

- IRIs d'espace de noms de vocabulaire, formes de termes (term shapes) et toute règle de validation
  spécifique au profil.

- Codecs, clés, algorithmes de signature, ancres de confiance ou hypothèses de déploiement requis.

- Taxonomie des défaillances : quelles violations sont des erreurs, des avertissements ou des
  diagnostics informatifs pour un outil sensible au profil.

- Interaction avec les segments, composition `cat`, suppression, compaction et blobs GTS
  imbriqués.

- Vecteurs de conformité, incluant le comportement lié aux profils non pris en charge pour les
  lecteurs (readers) de base.

- Considérations relatives à la sécurité et à la vie privée.

Une définition de profil DOIT (MUST) stipuler qu'elle ne modifie pas la grammaire de l'en-tête/trame (header/frame), la détection des limites de segment, les préimages de content-id ou de signature/hachage, la résolution du catalogue de transformations ou la sémantique de repli (fold) de base au §7. Le nouveau comportement du profil doit être exprimé sous forme de vocabulaire de graphe, de types de trames (frame types) existants, de capacités de transformation, de métadonnées ou de règles de validation sensibles au profil.

La politique de changement du registre, les espaces de noms réservés et le processus de promotion des profils standards optionnels sont maintenus dans [`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md).

<a id="131-language-tag-discipline-profile-level-normative"></a>

### 13.1 Discipline des étiquettes de langue (normative au niveau du profil)

Cette sous-section définit une règle pour le rédacteur (writer) de profil/projection, et non une exigence pour le lecteur (Reader) de base.

La charge utile de graphe d'un producteur PEUT (MAY) transporter des **étiquettes de langue internes à usage privé** (par exemple, `x-gmeow-*` de GMEOW) : la charge utile d'un segment `dist` ou `ai-package` *est* la forme canonique, et les formes canoniques conservent leurs étiquettes internes. Chaque **section de projection** — blocs de données (blobs) de documentation, vues dérivées, représentations projetées vers le bas, tout ce qui est généré *pour un consommateur externe* — DOIT (MUST) porter uniquement des **étiquettes BCP 47 publiques** ; un producteur qui laisse échapper des étiquettes à usage privé dans une section de projection DOIT (MUST) échouer au moment de l'écriture, et non simplement avertir (vecteur 20). La frontière est définie par *rôle*, et non par fichier : un paquet peut légitimement transporter une charge utile canonique avec des étiquettes internes à côté de sections de documentation avec des étiquettes publiques. (Ceci reflète la barrière contre les fuites d'étiquettes internes du cadre de générateur GMEOW ; le producteur de référence réutilise son mécanisme `retag` à la frontière de la section.)

<a id="132-the-files-profile-optional-standard"></a>

### 13.2 Le profil `files` (standard facultatif)

Le profil `files` est une archive de système de fichiers, adressée par contenu, définie comme
standard facultatif. C'est la réponse de GTS à `c`/`x`/`d` de tar : empaqueter un répertoire dans un
GTS à segment unique, le dépaqueter plus tard, et le comparer avec `diff` à un répertoire sans
comparaison octet par octet. Les règles ci-dessous sont des exigences de conformité au niveau du
profil pour les rédacteurs et validateurs `files`; un lecteur de base replie le graphe sans mettre
en oeuvre l'outillage d'archive.

**Espace de noms.** Le profil possède un petit vocabulaire défini par la spécification à
`https://w3id.org/gts/files#` (préfixe `files`). L'indépendance de GTS signifie qu'un outil de
dépaquetage NE DOIT PAS (MUST NOT) exiger GMEOW, schema.org, ni aucune autre ontologie pour lire
l'archive; le vocabulaire est rédigé dans la spécification et transporté comme IRIs littéraux dans
le graphe.

| terme | IRI | forme |

|---|---|---|

| `FileEntry` | `https://w3id.org/gts/files#FileEntry` | Classe. Une entrée archivée. |

| `path` | `https://w3id.org/gts/files#path` | Chaîne de chemin relatif, séparateurs `/`, aucun `/` initial, aucun composant `..`. |

| `digest` | `https://w3id.org/gts/files#digest` | Condensé de contenu `blake3:<hex>` des octets du fichier. |

| `size` | `https://w3id.org/gts/files#size` | Taille en octets comme `xsd:integer`. |

| `mode` | `https://w3id.org/gts/files#mode` | Bits de permission POSIX comme `xsd:integer` décimal (par exemple, `420` pour `0o644`). Les bits de type de fichier ne sont pas enregistrés. |

| `modified` | `https://w3id.org/gts/files#modified` | Heure de modification comme `xsd:dateTime` en UTC. |

| `mediaType` | `https://w3id.org/gts/files#mediaType` | Chaîne de type média IANA déclarée. |

| `type` | `https://w3id.org/gts/files#type` | Enum chaîne v2 : `file`, `directory`, `symlink`, `hardlink`, `fifo`, `chardev`, `blockdev` ou `socket`. L'absence signifie `file`. |

| `linkTarget` | `https://w3id.org/gts/files#linkTarget` | Chaîne brute de cible de lien symbolique v2, ou chemin d'archive cible de lien matériel. |

| `uid` / `gid` | `https://w3id.org/gts/files#uid`, `https://w3id.org/gts/files#gid` | Identifiants numériques de propriétaire v2 comme `xsd:integer`. |

| `userName` / `groupName` | `https://w3id.org/gts/files#userName`, `https://w3id.org/gts/files#groupName` | Noms de propriétaire v2 issus des métadonnées tar/PAX. |

| `devMajor` / `devMinor` | `https://w3id.org/gts/files#devMajor`, `https://w3id.org/gts/files#devMinor` | Numéros de périphérique v2 pour `chardev` et `blockdev`. |

| `xattr` | `https://w3id.org/gts/files#xattr` | Lien v2 vers un noeud blanc d'attribut. |

| `xattrName` / `xattrValue` | `https://w3id.org/gts/files#xattrName`, `https://w3id.org/gts/files#xattrValue` | Nom d'attribut étendu v2 et valeur lexicale base64. |

| `paxRecord` | `https://w3id.org/gts/files#paxRecord` | Lien v2 vers un noeud blanc d'échappatoire PAX textuel. |

| `paxKey` / `paxValue` | `https://w3id.org/gts/files#paxKey`, `https://w3id.org/gts/files#paxValue` | Clé et chaînes de valeur PAX inconnues v2, préservées pour des allers-retours tar sans perte. |

**Versions de profil.** La surface v1 est le profil minimal de fichiers réguliers ci-dessus. Une
archive v2 déclare `profileVersion: 2` dans les métadonnées d'en-tête et DEVRAIT (SHOULD) aussi
porter la même valeur dans une trame `meta` repliée afin que les outils conscients du profil
puissent la détecter après la lecture. Les lecteurs DOIVENT (MUST) traiter une absence de
`files:type` comme `file`, de sorte que les archives v1 continuent à se replier et à se dépaqueter
sous un lecteur v2. Les rédacteurs DEVRAIENT (SHOULD) émettre v2 seulement lorsque l'appelant le
demande explicitement ou lorsque des métadonnées non-v1 sont présentes.

**Forme des quads.** Chaque fichier régulier dans une archive v1 est décrit par un noeud blanc
`FileEntry` :

```text
_:entry a files:FileEntry ;
    files:path "relative/path.txt" ;
    files:digest "blake3:<hex>" ;
    files:size 1234 ;
    files:mode 33204 ;
    files:modified "2026-06-10T20:00:00Z"^^xsd:dateTime ;
    files:mediaType "text/plain" .
```

v2 utilise la même forme de sujet pour chaque type d'entrée. Les fichiers réguliers portent
digest/size et peuvent porter des propriétaires, xattrs ou lignes PAX. Les répertoires portent
`files:type "directory"` plus le chemin et les métadonnées, mais aucun digest ni taille. Les liens
symboliques et les liens matériels portent `files:linkTarget` et aucun blob. Les entrées spéciales
portent `files:type` et, pour les périphériques, `files:devMajor`/`files:devMinor` :

```text
_:dir a files:FileEntry ;
    files:path "empty" ;
    files:type "directory" ;
    files:mode 493 ;
    files:modified "2026-06-10T20:00:00.123456789Z"^^xsd:dateTime .

_:link a files:FileEntry ;
    files:path "current" ;
    files:type "symlink" ;
    files:linkTarget "releases/current" .

_:file a files:FileEntry ;
    files:path "data/events.csv" ;
    files:type "file" ;
    files:digest "blake3:<hex>" ;
    files:size 1234 ;
    files:xattr _:x0 ;
    files:paxRecord _:p0 .
_:x0 files:xattrName "user.comment" ;
    files:xattrValue "dmVyaWZpZWQ=" .
_:p0 files:paxKey "SCHILY.dev" ;
    files:paxValue "opaque" .
```

**Déterminisme.** Une archive `files` DOIT (MUST) être reproductible au bit près pour le même arbre d'entrée :

- Les chemins sont triés de manière lexicographique selon leur séquence d'octets UTF-8 avant l'émission.

- Les chemins stockés utilisent des séparateurs `/` et DOIVENT (MUST) être des chemins relatifs non vides. Les rédacteurs, les outils de désarchivage et les outils de comparaison DOIVENT (MUST) refuser les chemins absolus, les chemins relatifs au lecteur Windows, les composants `..`, les composants `.`, les composants vides et les séparateurs de barre oblique inverse avant de toucher aux octets des fichiers.

- Les heures de modification v1 sont normalisées en UTC et sérialisées en tant que `xsd:dateTime` avec une précision à la seconde. Les fractions de seconde DOIVENT (MUST) être tronquées avant l'émission v1. La v2 permet des fractions de seconde canoniques à largeur fixe lorsque le format source les fournit.

- La v1 n'enregistre que les fichiers réguliers, le mode POSIX et mtime. La v2 enregistre les métadonnées de portabilité tar requises pour des allers-retours sans perte : type d'entrée, cibles de liens, répertoires explicites, uid/gid, noms de propriétaires, numéros de périphériques, xattrs et enregistrements PAX inconnus. Les ACL restent en dehors du cœur de la v2, à moins qu'elles ne soient transportées en tant que données PAX ou xattr.

- Les `pack` et `diff` v1 DOIVENT (MUST) refuser les entrées de liens symboliques plutôt que de les suivre. Les lecteurs v2 peuvent préserver les métadonnées des liens symboliques et des fichiers spéciaux, mais l'extraction reste risquée par défaut (refuse-dangerous). `unpack` DOIT (MUST) refuser toute échappée de destination via un lien symbolique existant sous le répertoire de sortie.

- Les entrées v2 sont triées par chemin, les xattrs sont triés par nom puis par valeur, et les enregistrements PAX sont triés par clé puis par valeur avant l'émission.

**Blobs en ligne et externes.** Les octets d'un fichier régulier PEUVENT (MAY) être transportés en tant que trame (frame) `blob` en ligne (`"d"` présent, condensé (digest) = BLAKE3(`"d")` décodé)) ou en tant que blob externe (`"d"` absent, `pub.digest` nomme les octets détenus ailleurs, §12). Des octets identiques apparaissant sous plusieurs chemins sont stockés une seule fois par convention. La commande `gts pack` standard émet uniquement des blobs en ligne. Les implémentations PEUVENT (MAY) ajouter un mode de blob externe explicite, mais il DOIT (MUST) être optionnel (opt-in) et documenté. `gts unpack` DOIT (MUST) refuser un `FileEntry` non supprimé dont le blob en ligne est absent ; `gts diff` PEUT (MAY) comparer par `files:digest` sans récupérer les octets externes.

Les répertoires, liens symboliques, liens physiques (hardlinks), fifos, nœuds de périphériques et sockets NE DOIVENT PAS (MUST NOT) transporter `files:digest` ou `files:size`. Le `files:linkTarget` d'un lien physique est un autre chemin d'archive ; le `files:linkTarget` d'un lien symbolique est la charge utile brute du lien.

**Suppression.** Une suppression ciblée sur un blob (§11) masque les octets du fichier correspondant de l'extraction par défaut. `gts unpack` et `gts extract` ignorent/refusent les blobs supprimés par défaut et exposent une dérogation `--include-suppressed` explicite lorsque l'opérateur souhaite intentionnellement conserver l'historique.

**Relation avec d'autres vocabulaires.** Le profil est délibérément autonome, mais les termes s'alignent par référence à des vocabulaires de surface courants : `files:size` ↔ schema.org `contentSize`, `files:mediaType` ↔ schema.org `encodingFormat`, `files:modified` ↔ NFO `fileLastModified`, `files:path` ↔ NFO `fileName`. Ces alignements vivent dans le DSL de mappage de GMEOW ; le profil de fichiers lui-même n'en dépend pas.

<a id="133-the-stream-vocabulary-optional-standard"></a>

### 13.3 Le vocabulaire `stream` (standard facultatif)

L'état de disposition diffusable en continu (§3.3) et la compaction diffusable en continu (§10.1)
utilisent un petit vocabulaire standard facultatif à `https://w3id.org/gts/stream#` (préfixe
`stream`) — le même choix d'indépendance que pour le profil `files` (§13.2) : aucune ontologie
GMEOW ni externe n'est requise pour diffuser une archive photo; les termes sont rédigés ici et
transportés comme IRIs littéraux dans le graphe. Le vocabulaire est volontairement distinct de
`files#` (les deux se composent : une archive `files` qui est aussi diffusable en continu décrit
chaque fichier une fois comme `files:FileEntry` et une fois comme `stream:Manifestation` — le
contrôle de profil (§14.1) et le contrôle de disposition (§3.3) restent indépendants).

**Termes d'index de diffusion en continu** — un `stream:Manifestation` par blob promis, émis dans
l'index de diffusion de tête avant toute trame `blob` (§3.3) :

| terme | IRI | forme |

|---|---|---|

| `Manifestation` | `https://w3id.org/gts/stream#Manifestation` | Classe. Un blob que ce segment promet de livrer. |

| `digest` | `https://w3id.org/gts/stream#digest` | Condensé de contenu `blake3:<hex>` — la reconnaissance de dette que le blob acquitte. |

| `mediaType` | `https://w3id.org/gts/stream#mediaType` | Type média IANA déclaré (miroir de `pub.mt` du blob). |

| `size` | `https://w3id.org/gts/stream#size` | Taille en octets du blob décodé comme `xsd:integer`. |

| `role` | `https://w3id.org/gts/stream#role` | Chaîne de rôle de livraison : `"preview"` / `"primary"` / `"source"`; ensemble ouvert. |

| `order` | `https://w3id.org/gts/stream#order` | Position de livraison prévue parmi les blobs du segment, `xsd:integer`, à base 0. |

**Termes de provenance de compaction** — le vocabulaire concret pour le MUST de provenance de
§10/§10.1 :

| terme | IRI | forme |

|---|---|---|

| `Compaction` | `https://w3id.org/gts/stream#Compaction` | Classe. Un événement de réécriture (un noeud blanc). |

| `agent` | `https://w3id.org/gts/stream#agent` | L'outil agissant, une chaîne (p. ex. `"gts-compact"`). |

| `timestamp` | `https://w3id.org/gts/stream#timestamp` | Heure de réécriture comme `xsd:dateTime` en UTC. |

| `sourceHead` | `https://w3id.org/gts/stream#sourceHead` | Identifiant de tête `blake3:<hex>` d'un segment source; répété par segment. |

| `sealedSource` | `https://w3id.org/gts/stream#sealedSource` | Condensé `blake3:<hex>` du blob GTS imbriqué contenant l'original textuel (§10.1). |

| `DetachedSignature` | `https://w3id.org/gts/stream#DetachedSignature` | Classe. Une signature de trame reportée (un noeud blanc). |

| `sourceFrame` | `https://w3id.org/gts/stream#sourceFrame` | Trame originale `"id"` `blake3:<hex>` contre laquelle la signature COSE se vérifie, pour toujours. |

| `cose` | `https://w3id.org/gts/stream#cose` | Les octets COSE_Sign1 originaux, littéral base64url (sans bourrage). |

**Forme des quads** (index de diffusion d'un segment compacté, puis provenance) :

```text
_:m0 a stream:Manifestation ;
    stream:digest "blake3:<hex>" ;
    stream:mediaType "image/webp" ;
    stream:size 20480 ;
    stream:role "primary" ;
    stream:order 0 .
_:c a stream:Compaction ;
    stream:agent "gts-compact" ;
    stream:timestamp "2026-01-01T00:00:00Z"^^xsd:dateTime ;
    stream:sourceHead "blake3:<hex>" .
_:s0 a stream:DetachedSignature ;
    stream:sourceFrame "blake3:<hex>" ;
    stream:cose "<base64url>" .
```

**Couplage de revendication (normatif).** L'utilisation de termes `stream#` dans un segment qui ne
revendique PAS (NOT) `"layout": "streamable"` est un **avertissement**, pas une erreur (§14.1) :
les quads de provenance survivent légitimement aux allers-retours `gts → nq → gts` et à la
ré-accrétion après des ajouts. La classe d'erreur est réservée à la dérive inverse — une disposition
revendiquée que les octets contredisent (§3.3).

<a id="134-domain-profile-example-music-package-informative"></a>

### 13.4 Exemple de profil de domaine : `music-package` (informatif)

Cette sous-section est un exemple informatif d'un profil propre à un domaine. Un lecteur de base,
un rédacteur ou un vérificateur n'est pas tenu de mettre en oeuvre le vocabulaire GMEOW, les règles
du domaine musical, les règles de projection de notation ni le validateur `music-package` pour être
conforme au noyau GTS.

Le profil `music-package` peut être défini comme un GTS à segment unique qui transporte du contenu
musical relatif aux cadres : un `MusicalWork`/`MusicalExpression`, ses `Voice`s et
`MusicalSegment`s, les cadres de référence `TuningSystem` et `MusicalTimeFrame`, les `ToneEvent`s
atomiques, les déclarations `DegreeOfFreedom` et les revendications d'analyse indexées par point
de vue. C'est la forme de transport canonique pour la tranche musicale GMEOW et l'entrée des
projections de notation.

**Espace de noms.** Le profil réutilise le vocabulaire musical GMEOW
(`https://blackcatinformatics.ca/gmeow/`). Un `music-package` n'est pas obligé d'être un profil
`dist` : il peut ne transporter que le graphe de contenu musical plus les blobs de projection, et il
PEUT (MAY) s'appuyer sur un instantané `dist` externe pour les définitions de vocabulaire.

**En-tête.** Un segment `music-package` déclare `"prof": "music-package"`. Le profil peut être
append-only pour de nouvelles revendications; les triples existants ne sont pas supprimés, seulement
supplantés par la provenance au niveau des énoncés (§7.3).

**Exemple de forme de quads.** Un paquet minimal peut contenir :

```text
@prefix gmeow: <https://blackcatinformatics.ca/gmeow/> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .

:piece a gmeow:MusicalExpression ;
    gmeow:hasVoice :voice1 .

:voice1 a gmeow:Voice ;
    gmeow:voiceTuningFrame :tuning12EDO ;
    gmeow:voiceTimeFrame :timeGrid .

:tuning12EDO a gmeow:TuningSystem .
:timeGrid a gmeow:MusicalTimeFrame .

:event1 a gmeow:ToneEvent ;
    gmeow:segmentOf :voice1 ;
    gmeow:toneEventPitchValue :pitchC4 ;
    gmeow:segmentSpan :span1 .

:span1 a gmeow:MusicalTimeSpan ;
    gmeow:hasMusicalTimeFrame :timeGrid ;
    gmeow:timeStartNumerator 0 ;
    gmeow:timeStartDenominator 1 ;
    gmeow:timeDurationNumerator 1 ;
    gmeow:timeDurationDenominator 4 .
```

Le temps et la hauteur sont **relatifs aux cadres** : `toneEventPitchValue` pointe vers une
`PitchValue` interprétée sous le cadre d'accord de la voix de l'événement, et les décalages/durées
sont des valeurs rationnelles interprétées sous le cadre temporel de la voix.

**Projections.** Un `music-package` peut contenir des trames `blob` dont les octets sont des
représentations projetées vers le bas (MusicXML, MEI, ABC, LilyPond, Humdrum **kern, MIDI, Scala
`.scl`, tablature, notation mensurale, notation graphique). Un validateur du profil music-package
peut exiger que chaque projection soit accompagnée d'un manifeste de pertes déclarées qui énumère le
`NotationProjectionProfile` utilisé, les `MusicalParameter`s qu'il peut représenter et les
`ProjectionLoss`es qu'il entraîne. Le manifeste peut être un fichier d'accompagnement Turtle ou un
en-tête/commentaire intégré, et il est considéré comme partie de la projection, non du graphe
canonique.

**Couplage avec le profil bundle.** Un profil `bundle` (§12.1) dont les blobs sont des segments
`music-package` fournit le cas de transport multi-mouvement / multi-version. Chaque segment imbriqué
conserve sa propre déclaration de profil; le bundle externe n'impose pas de conventions
supplémentaires.

**Vérification.** Un vérificateur conscient de music-package peut vérifier que chaque
`NotationSystem` référencé par un blob de projection possède un `NotationProjectionProfile`
correspondant, et que le profil rend compte de chaque `MusicalParameter` déclaré dans la tranche
musicale (aucune omission silencieuse). Le `gts verify` de base n'est pas tenu de mettre en oeuvre
ce profil; il peut signaler le profil non pris en charge sans échouer la validité du format filaire.

<a id="14-transforms-out"></a>

## 14. Transformations sortantes

Les transformations convertissent le GTS en substrats d'exploitation. Chacune est une mince couche d'adaptation sur les tables repliées — aucun analyseur de texte RDF n'est impliqué.

- `gts → nquads` / `gts → turtle` — sérialisent `quads` + `reifies`/`annot` (ce dernier sous forme de réification RDF 1.2). Les blobs en ligne sont **externalisés** vers `./blobs/<blake3>.bin`, et les références de condensé du graphe se résolvent vers ces chemins. Les trames opaques se sérialisent en tant que leurs descriptions de nœuds opaques.

- `gts → duckdb` / `gts → sqlite` — chargent en bloc les quatre tables (`terms`, `quads`, `reifies`, `annot`) plus une table `blobs` ; créent les index appropriés au moteur. Il s'agit d'un chargement quasi mécanique car les tables GTS correspondent déjà à la forme relationnelle.

Chaque transformation DEVRAIT (SHOULD) être vérifiable par une **équivalence aller-retour** : pour les trames **entièrement décodables**, `gts → nq → gts` DOIT (MUST) produire le même graphe replié (modulo l'étiquetage des nœuds blancs et le ré-encodage CBOR déterministe). Les nœuds opaques sont exclus — ils se sérialisent sous forme de descriptions de nœuds opaques et se réimportent sous forme de quads ordinaires, et non de trames opaques.

<a id="141-composition-tooling-requirements-normative-for-conformant-tools"></a>

### 14.1 Exigences relatives aux outils de composition (normative pour les outils conformes)

Cette section définit uniquement la conformité des outils. Un lecteur de base (`Baseline Reader`) ou un rédacteur (`Writer`) n'a pas besoin d'inclure ces verbes CLI, cibles de transformation, commandes d'archive ou politiques de publication pour être conforme au noyau (`core-conformant`). Les outils tenant compte des profils (`Profile-aware`) n'appliquent que les validateurs de profil qu'ils prétendent prendre en charge ; les profils non pris en charge sont présentés comme des diagnostics ou des métadonnées, à moins que l'utilisateur n'ait explicitement demandé la validation de ce profil.

Le `cat` brut fonctionne toujours (§3.1) ; un compositeur de validation conforme (`gts cat`) et un vérificateur (`gts verify`) ajoutent la posture de refus par défaut (« refuse-don't-trust ») :

- **`gts cat` DOIT (MUST) refuser les entrées dégénérées** : une entrée qui n'est pas un GTS valide, un segment dont le repli (`fold`) produit zéro quads et zéro blobs (presque toujours un bogue de câblage, jamais un véritable paquet), ou une sortie dans laquelle un segment de suppression uniquement masquerait chaque trame précédente. Les outils de classe publication ne font jamais confiance à un état pathologique comme étant intentionnel.

- **`gts verify` DOIT (MUST) vérifier les exigences déclarées par rapport aux calculées pour les profils pris en charge** : un segment dont le graphe utilise le vocabulaire d'un profil pris en charge sans déclarer le profil constitue une **erreur** ; un profil pris en charge déclaré mais non utilisé est un avertissement. Les déclarations qu'un outil lit (le rapport de dépendance CLI, §13) ne doivent pas pouvoir se détériorer par rapport au contenu qu'elles décrivent.

- **`gts verify` DEVRAIT (SHOULD) effectuer un rapport par segment** : identifiant d'en-tête, ensemble de signataires, profil, décomptes de termes/quads, décompte de nœuds opaques avec motifs — le registre de composition du fichier.

- **`gts verify` DOIT (MUST) vérifier la revendication de disposition** (§3.3) : un segment revendiquant `"layout": "streamable"` dont la région couverte viole l'ordre de livraison, ou dont le pied de page d'index est manquant ou contredit les trames qu'il couvre, est une **erreur** (`StreamableLayoutError`, §2.3) ; le vocabulaire `stream#` dans un segment non revendiqué est un **avertissement** (§13.3). `gts info` et `gts verify` DEVRAIENT (SHOULD) signaler la limite diffusable en continu d'un segment revendiqué — « diffusable en continu jusqu'à la trame *N*, queue accrétive de *M* trame(s) ».

- **`gts compact --streamable <in> -o <out>` est la réécriture de disposition** (§10.1). Elle DOIT (MUST) refuser une entrée qui ne se vérifie pas proprement, une entrée portant des suppressions adressées par trame, et une entrée `evidence` sans l'option de scellement de l'original (`--seal-original`, §10.1) ; elle DOIT (MUST) émettre un seul segment revendiqué dans la forme diffusable en continu normative (§3.3) avec provenance de compactage et signatures détachées (§13.3), et sa sortie DOIT (MUST) être déterministe au niveau des octets pour la même entrée et les mêmes paramètres (blobs ordonnés par taille décodée croissante, les égalités étant rompues par condensat (« digest ») croissant ; l'horodatage de réécriture est un paramètre, pas l'heure ambiante).

- **Mode de création de graphe déterministe** : il s'agit de la surface de rédacteur de construction reproductible pour un graphe replié. Il émet un segment ordinaire et DOIT (MUST) remapper les identifiants de termes locaux avant l'écriture : les termes sont triés par valeur sémantique (chaîne IRI ; forme lexicale littérale plus IRI de type de données effectif plus étiquette de langue ; étiquette de nœud vierge, les nœuds vierges anonymes utilisant leur occurrence d'entrée comme critère de départage ; triple cité résolu en sa valeur sujet/prédicat/objet). Il émet ensuite des trames créables dans cet ordre fixe : `terms`, `quads`, `reifies`, `annot`, `blob`, `meta`, `suppress`. Les quads, les liaisons de réificateur, les annotations, les blobs, les clés de métadonnées et les trames de suppression sont triés par la représentation CBOR déterministe remappée. Le mode ne rejoue pas les observations du lecteur (`opaque`, signatures, diagnostics ou registres de segments) ; les outils de publication qui doivent préserver ces observations doivent utiliser une réécriture spécifique au profil telle que le compactage diffusable en continu ou sceller les octets originaux comme preuve.

- **L'extraction de blob est une vérification, jamais une conversion** (`gts ls`, `gts extract`) : les blobs sont adressés par condensat de contenu (les indices de trame sont des accidents physiques qui se déplacent sous `cat`) ; l'extraction recalcule le hachage des octets par rapport au condensat demandé ; un blob supprimé par condensat (§11) est refusé par défaut (la suppression est un contrat d'affichage et l'extraction est de l'affichage) avec une dérogation explicite ; un indicateur de type de média est une **assertion** par rapport au `pub.mt` déclaré du blob — un outil de publication de validation refuse une non-correspondance plutôt que de procéder à un transcodage.

<a id="142-archive-tooling-files-profile"></a>

### 14.2 Outillage d'archivage (profil `files`)

Le profil `files` ajoute des commandes de publication de validation. Elles partagent la posture « refuser-ne-pas-faire-confiance » du §14.1 : les opérations sur les octets bruts sont toujours des GTS valides, mais un outil refuse les états pathologiques plutôt que de leur faire confiance comme étant intentionnels. La commande stable `pack` émet le profil regular-file v1 par défaut. Les métadonnées v2 sont une surface de création optionnelle (opt-in) pour les ponts tar et autres outils d'archivage sans perte.

- **`gts pack <dir|file>... -o out.gts`**
  Produire un GTS à segment unique dont l'en-tête déclare `"prof": "files"`. Chaque argument est archivé : un fichier est ajouté sous son nom de base (basename) ; un répertoire est ajouté récursivement en tant qu'entrées regular-file. Les répertoires vides et les entrées non standards ne sont pas inclus par cette commande v1. L'archive résultante contient, dans l'ordre, le `terms` et le `quads` décrivant chaque `files:FileEntry`, suivis des trames (frames) `blob` en ligne pour le contenu des fichiers. La commande DOIT (MUST) refuser :

  - les entrées contenant des chemins stockés non sécurisés : chemins absolus, chemins relatifs à un lecteur, `..`, `.`, composants vides ou séparateurs de barre oblique inverse ;

  - les liens symboliques (symlinks) ;

  - les entrées qui ne sont pas lisibles ou qui disparaissent pendant le parcours.

- **Aides à la création v2 / entrée de pont tar**
  Un outil qui revendique la prise en charge du profil de fichiers (files-profile) v2 PEUT (MAY) émettre des répertoires explicites, des liens symboliques (symlinks), des liens physiques (hardlinks), des fifos, des nœuds de périphérique, des sockets, la propriété, des xattrs et des enregistrements PAX en utilisant le vocabulaire v2 du §13.2. Il DOIT (MUST) marquer le segment avec `profileVersion: 2`, maintenir les entrées triées par chemin stocké, maintenir les xattrs/enregistrements PAX triés, et préserver la compatibilité v1 en omettant ou en définissant par défaut `files:type` à `file` lors de la lecture d'anciennes archives.

- **`gts unpack <archive> [-C dir]`**
  Écrire chaque `files:FileEntry` de l'archive dans le répertoire de destination (répertoire de travail actuel par défaut). La commande DOIT (MUST) :

  - refuser d'écrire en dehors du répertoire de destination (`..`, chemins absolus ou liens symboliques qui s'en échappent) ;

  - créer des répertoires v2 explicites, mais refuser l'extraction de liens symboliques, liens physiques, fifos, nœuds de périphérique et sockets à moins que l'utilisateur n'ait fourni `--allow-symlinks` ou `--allow-special` pour ces classes ; les cibles de liens symboliques restent confinées à l'arborescence de destination même après l'activation (opt-in) ;

  - recalculer le hachage de chaque fichier regular-file écrit et vérifier qu'il correspond à `files:digest` ;

  - restaurer l'heure de modification et les permissions déclarées de l'entrée (sous réserve du système d'exploitation hôte) ;

  - ne jamais changer la propriété à moins que l'utilisateur ne fournisse `--same-owner` ou un mécanisme d'activation (opt-in) privilégié équivalent tel que `--numeric-owner`, et ne jamais restaurer les bits setuid/setgid/sticky à moins que l'utilisateur ne fournisse `--preserve-setid` ;

  - ignorer par défaut les entrées dont l'empreinte (digest) est supprimée (§11), avec une dérogation explicite `--include-suppressed`.

- **`gts tar -c/-x/-t/-d`**
  Une interface en ligne de commande (CLI) compatible tar PEUT (MAY) envelopper `pack`, `unpack`, `diff`, `from-tar` et `to-tar` avec des drapeaux familiers (`-cf`, `-czf`, `--zstd`, `-xf`, `-tf`, `-df` et `-C`). L'enveloppe (wrapper) DOIT (MUST) préserver la même politique de sécurité que `unpack` : les opérations de liste/diff non mutantes peuvent inspecter les métadonnées de liens et de fichiers spéciaux, mais l'extraction nécessite toujours les activations explicites ci-dessus.
  Les outils DEVRAIENT (SHOULD) choisir le chemin `.gts` ou `.tar` selon l'extension de l'archive et DEVRAIENT (SHOULD) déduire l'enveloppement gzip/zstd à partir des suffixes tar courants lors de la création d'une sortie tar.
  Les outils qui revendiquent la diffusabilité en continu (streamability) de grandes archives DEVRAIENT (SHOULD) déclarer la limite exacte qu'ils satisfont : la création directe de `.gts` peut diffuser en continu les trames (frames) de charge utile de fichiers regular-file pendant que les métadonnées sont triées, mais les projections de graphes repliés (folded) et les backends de compression peuvent encore nécessiter un stockage temporaire limité ou une matérialisation en mémoire.

- **`gts diff <archive> <dir>`**
  Comparer l'ensemble `files:FileEntry` de l'archive à l'état actuel de `<dir>` par empreinte de contenu (digest). Signaler les chemins ajoutés, supprimés et modifiés. Quitter avec `0` si le répertoire correspond exactement à l'archive ; quitter avec `1` si un chemin diffère ou si l'entrée est refusée. Aucune comparaison d'octets n'est nécessaire : l'adressage par contenu rend l'opération O(lecture) sur le répertoire.

**Comparaison du flux de travail d'archivage.**

| flux de travail | table des matières habituelle | comportement du profil GTS `files` |

|---|---|---|

| `tar` | Les enregistrements d'en-tête sont entrelacés avec les octets du fichier ; l'interprétation du chemin et des métadonnées relève de la politique de l'outil. | Le manifeste v1 est constitué de quads RDF pour les fichiers réguliers. v2 ajoute des types d'entrée équivalents à tar, des cibles de liens, la propriété, des nœuds de périphérique, des xattrs et des enregistrements d'échappement PAX tout en gardant la politique d'extraction explicite. |

| `zip` | Le répertoire central permet l'accès aléatoire mais est un pied de page orienté vers la réécriture. | GTS reste en ajout uniquement ; des index optionnels accélèrent l'accès sans faire du pied de page l'identité de l'archive. |

| Paquet de style BagIt | Fichiers de charge utile plus manifestes/sommes de contrôle sidecar. | Le manifeste natif au graphe et les octets de contenu voyagent dans une CBOR Sequence vérifiable ; les blobs externes restent adressés par contenu lorsqu'ils sont utilisés. |

La proposition de valeur n'est pas le taux de compression. Utilisez une transformation de compression ou un transport externe lorsque la taille prédomine. Le profil `files` est destiné aux manifestes natifs au graphe, à la déduplication adressée par condensé, à la composition par ajout et à une politique de sécurité cohérente entre les moteurs.

<a id="15-worked-examples"></a>

## 15. Exemples détaillés

Le format CBOR est présenté en **notation de diagnostic** (RFC 8949 §8). Les hachages/signatures sont masqués par `h'…'`.

<a id="151-minimal-distribution-snapshot-dist"></a>

### 15.1 Instantané de distribution minimale (`dist`)

```text
55799(                                   / self-describe magic /
  { "gts": "GTS1", "v": 1, "prof": "dist",
    "cat": { 0: {"name":"identity","cls":"encode"},
             4: {"name":"zstd","cls":"compress"} },
    "id": h'…header.id…' }
)
{ "t": "terms", "prev": h'…header.id…', "id": h'…terms.id…',
  "d": [ {"k":0,"v":"https://example.org/Cat"},          / id 0 /
         {"k":0,"v":"http://www.w3.org/2000/01/rdf-schema#label"},  / id 1 /
         {"k":1,"v":"Cat","l":"en"} ] }                  / id 2 /
{ "t": "quads", "prev": h'…terms.id…', "id": h'…', "x": [4],
  "d": h'…zstd([[0,1,2]])…' }                            / Cat rdfs:label "Cat"@en /
```

Le terme 2 est un littéral avec une étiquette de langue et aucun `"dt"`, donc son type de données est `rdf:langString`
(§7.1).

<a id="152-evidence-image-signed-accrual-evidence"></a>

<a id="152-evidence-image--signed-accrual-evidence"></a>

### 15.2 Preuve : image + accroissement signé (`evidence`)

```text
{ "t": "blob", "prev": h'…header.id…', "id": h'…',
  "pub": {"mt":"image/jp2"}, "d": h'…image bytes…',      / digest = blake3(d) /
  "sig": h'COSE_Sign1 by did:photographer' }
{ "t": "annot", "prev": h'…blob.id…', "id": h'…',
  "d": [[10,11,12]],                                     / reifier 10: capturedAt … /
  "sig": h'COSE_Sign1 by did:photographer' }
{ "t": "annot", "prev": h'…prev.id…', "id": h'…',        / later custody transfer, separate signer /
  "pub": {"event":"custody-transfer"},
  "d": [[13,11,14]], "sig": h'COSE_Sign1 by did:evidence-clerk' }
```

Rien n'est réécrit ; chaque accroissement est lié par hachage et signé indépendamment.

<a id="153-notary-partially-opaque-frame-opaque"></a>

### 15.3 Notary : trame partiellement opaque (`opaque`)

```text
{ "t": "annot", "prev": h'…prev.id…', "id": h'…',
  "pub": { "claim": "I hereby notarized this document.",
           "notary": "did:notary:jane", "ts": "2026-06-09T12:00:00Z" },
  "x": [4, 7],                                            / 7 = cose-encrypt /
  "to": [ {"kid":"anon:7f3a…","alg":"ECDH-ES+A256KW"} ],  / pseudonymous kid (opaque profile, §18) /
  "d": h'COSE_Encrypt(verified ID record + provenance)',
  "sig": h'COSE_Sign1 by did:notary:jane' }
```

Quiconque vérifie la notarisation publique et sa signature ; seule la clé du tribunal déchiffre l'enregistrement scellé ; la signature lie les deux (§9.2). Un lecteur sans la clé du tribunal replie ceci en un noeud opaque avec `reason:"missing-key"`, `pub` intacts, `sigstat:"valid"`.

<a id="154-graceful-degradation-image-content-negotiation"></a>

### 15.4 Dégradation gracieuse (`image`, négociation de contenu)

```text
{ "t": "blob", "prev": h'…', "id": h'…', "pub": {"mt":"image/vnd.djvu","rep":"master"}, "x":[9], "d": h'…' }
{ "t": "blob", "prev": h'…', "id": h'…', "pub": {"mt":"image/jpeg","rep":"fallback"}, "d": h'…' }
```

Un lecteur ne disposant pas du codec `9` (djvu) replie le maître en un noeud opaque et utilise la solution de repli JPEG — les deux sont présents, les deux sont liés par hachage.

<a id="155-matryoshka-a-whole-signed-gts-sealed-inside-a-frame-bundle-opaque"></a>

<a id="155-matryoshka-a-whole-signed-gts-sealed-inside-a-frame-bundle--opaque"></a>

### 15.5 Matryoshka : un GTS signé complet scellé à l'intérieur d'une trame (`bundle` / `opaque`)

```text
{ "t": "blob", "prev": h'…', "id": h'…',
  "pub": { "rep": "sealed-evidence-graph", "mt": "application/vnd.blackcat.gts+cbor-seq" },
  "x": [4, 7],                                            / zstd then cose-encrypt /
  "to": [ {"kid":"did:court:registry"} ],
  "d": h'COSE_Encrypt( zstd( <a complete, independently-signed GTS file> ) )' }
```

Sans la clé de la cour, ceci se replie en un seul nœud opaque — un sous-graphe complet que le détenteur transporte mais ne peut lire, et pourtant dont la présence et la position sont prouvées par la chaîne externe. Avec la clé, un Lecteur Complet (Full Reader) effectue une récursion (§12.1) et replie le GTS interne — en-tête, chaîne, signatures et tout le reste — en un sous-graphe vérifiable.

<a id="16-media-type-and-http-serving-contract"></a>

## 16. Type de média et contrat de service HTTP

Les fichiers GTS sont des artefacts publiés. Cette section définit la conformité du déploiement : un fichier GTS stocké localement peut être valide au format filaire, conforme au lecteur et conforme au rédacteur même lorsqu'il n'est jamais servi via HTTP. Un déploiement conforme DOIT (MUST) annoncer le type de média, prendre en charge les requêtes par plage et définir des en-têtes de cache qui respectent l'immutabilité du format.

<a id="161-media-type-and-file-extension-normative"></a>

### 16.1 Type de média et extension de fichier (normative)

- **Type de média :** `application/vnd.blackcat.gts+cbor-seq` (modèle d'enregistrement au §20.1).
  GTS utilise le suffixe de syntaxe structurée `+cbor-seq` car un fichier GTS est une séquence CBOR
  ([RFC 8742]) d'en-têtes de segment et de trames, et non un seul élément de données CBOR. L'orthographe
  provisoire antérieure `application/vnd.blackcat.gts+cbor` est obsolète ; les déploiements DOIVENT émettre
  (MUST) `application/vnd.blackcat.gts+cbor-seq`. Les lecteurs PEUVENT accepter (MAY) l'orthographe obsolète
  comme alias hérité, mais NE DOIVENT PAS émettre (MUST NOT) celle-ci dans les métadonnées nouvellement écrites.

- **Extension de fichier :** `.gts`.

- **Octets magiques :** l'étiquette d'auto-description CBOR `55799` (`0xd9 0xd9 0xf7`) au début de
  l'en-tête (Header) du premier segment lorsque le premier segment est étiqueté. Un lecteur PEUT utiliser
  (MAY) ces trois octets comme signal lors de l'identification d'un fichier GTS potentiel, mais DOIT
  confirmer (MUST) la forme de l'en-tête (Header) avant de traiter les octets comme étant du GTS.

Les serveurs qui ne reconnaissent pas `application/vnd.blackcat.gts+cbor-seq` DEVRAIENT se replier (SHOULD) sur
`application/octet-stream` plutôt que sur un type de texte erroné ; les clients DEVRAIENT inspecter
(SHOULD) le premier élément de données CBOR lorsque le type de média est manquant ou générique.

<a id="162-file-identification-algorithm-normative"></a>

### 16.2 Algorithme d'identification de fichier (normatif)

Les métadonnées de type de média font autorité lorsqu'elles sont disponibles. Lorsqu'un lecteur (reader) doit identifier des octets sans métadonnées fiables, il DOIT (MUST) utiliser cet algorithme :

1. Traiter `.gts` et `application/octet-stream` comme des indices seulement ; ni l'un ni l'autre ne prouve ou n'infirme GTS.

2. Si les trois premiers octets sont `0xd9 0xd9 0xf7`, analyser le premier élément CBOR en tant qu'élément étiqueté et déballer l'étiquette `55799`. Sinon, analyser le premier élément CBOR à partir du décalage d'octets `0`.

3. Le premier élément déballé DOIT (MUST) être une carte d'en-tête (Header map) contenant `"gts": "GTS1"` et dépourvue de la clé de trame (frame) `"t"`. Une non-correspondance n'est pas un fichier GTS.

4. Une identification positive n'est encore qu'un résultat d'identification. La validité complète nécessite l'analyse de l'ensemble du flux d'octets observé en tant que séquence CBOR (§3), l'application des règles de délimitation de segment (segment) (§3.1) et la validation des ids, des chaînes (chains), des profils (profiles) et des capacités (capabilities) tel que requis par la classe de conformité sélectionnée.

5. Les implémentations NE DOIVENT PAS (MUST NOT) exiger une enveloppe CBOR pour l'ensemble du fichier, un compte total d'éléments ou un préfixe de longueur. Des segments (segments) étiquetés valides de manière indépendante peuvent être concaténés, de sorte que les étiquettes `55799` ultérieures identifient les en-têtes de segment ultérieurs, et non des objets de fichier complet imbriqués.

<a id="163-http-serving-semantics-normative"></a>

### 16.3 Sémantique de service HTTP (normatif)

Un paquet GTS est servi comme toute autre version binaire immuable, avec trois exigences supplémentaires :

1. **`Accept-Ranges: bytes`** DOIT (MUST) être envoyé pour chaque réponse `.gts`. Le format est conçu pour une consommation partielle et diffusable en continu (§3.2) : un consommateur peut effectuer un repli (fold) de l'en-tête et d'un préfixe de trames sans télécharger l'intégralité du fichier. Les clients choisissent des plages d'octets à partir de décalages d'éléments CBOR découverts, d'index ou d'autres manifestes de confiance ; la prise en charge des plages HTTP ne valide ni ne répare en soi les octets du fichier local.

2. **Aucune transformation à la périphérie.** Étant donné que les octets constituent une chaîne adressée par le contenu, les proxys et les serveurs NE DOIVENT PAS (MUST NOT) appliquer de compression, de minification ou toute transformation altérant les octets. Les trames sont déjà compressées par le codec choisi par le rédacteur (writer) ; une recompression au niveau de la couche de transport rompt les hachages de contenu et les signatures.

3. **CORS.** Un paquet de vocabulaire/jeu de données public est censé être lisible de manière cross-origin. Les réponses DEVRAIENT (SHOULD) inclure `Access-Control-Allow-Origin: *` pour l'origine `.gts` servie.

<a id="164-immutability-aware-caching-normative"></a>

### 16.4 Mise en cache sensible à l'immutabilité (normative)

Les versions GTS publiées sont immuables ; une URL de paquet GTS désigne une séquence d'octets exacte.

- **URL versionnées** (`…/gmeow/1.2.3/gmeow.gts`, `…/packages/music/2026-06-18/music.gts`, ou toute
  URL contenant un identifiant de version/date/tête) DOIVENT (MUST) être servies avec :

  ```text
  Cache-Control: public, max-age=31536000, immutable
  ETag: "<last-segment-head>"
  ```

  L'ETag naturel est l'hexadécimal de l'identifiant de tête du dernier segment (§3.1) du fichier, car il engage de manière transitive chaque octet du fichier. La directive `immutable` indique aux caches qu'ils n'ont pas besoin de revalider pour la durée de vie d'un an.

- **`latest` / alias conneg** (URL qui se résolvent vers la version actuelle et peuvent changer) NE DOIVENT PAS (MUST NOT) être mis en cache en tant que variante unique :

  ```text
  Cache-Control: private, no-store
  Vary: Accept
  ```

  Le `Vary: Accept` empêche l'empoisonnement du cache conneg lorsque le même chemin négocie vers HTML, Turtle ou le paquet GTS. Il s'agit de la même classe d'empoisonnement de cache traitée pour les IRI de tranche (slice) par le générateur Apache.

La sélection de profil (profil) reste de forme URL dans la v0.2 : une URL par paquet. RFC 6906 / `Accept-Profile`
est noté comme une extension future possible, non requise pour la conformité v0.2.

<a id="17-versioning-and-durability-guarantees"></a>

## 17. Versions et garanties de durabilité

- L'en-tête `"v"` est la version majeure de la spécification. Un lecteur DOIT (MUST) refuser une version majeure qu'il n'implémente pas, mais DOIT (MUST) tout de même vérifier la chaîne id/prev et énumérer les types/identifiants de trame.

- **Sémantique des segments et lecteurs plus anciens.** Un lecteur implémentant cette révision DOIT (MUST) prendre en charge les limites de segment (§3.1). Un lecteur qui ne le fait PAS (une implémentation antérieure au §3.1) rencontre un second en-tête en tant qu'élément de données autre qu'une trame : une telle entrée est **malformée pour ce lecteur**, et il DOIT (MUST) produire un diagnostic fatal pour le reste du fichier plutôt que d'ignorer l'élément — *un repli erroné silencieux (appliquer des identifiants de termes globaux au fichier à travers une limite) est le seul résultat interdit* (vecteur 17). Étant donné que `cat` ne peut pas réécrire l'en-tête du premier segment (l'auto-hachage le scelle), les fichiers multi-segments ne peuvent pas s'annoncer dans le premier en-tête ; la détection des limites est donc structurelle, et la règle d'échec critique est ce qui protège les lecteurs les plus anciens de l'écosystème.

- **Durabilité de la structure :** un fichier GTS combiné à cette spécification est décodable à jamais sans moteur ni dictionnaire externe — CBOR est une norme IETF et les dictionnaires sont intégrés (in-band).

- **Durabilité de la densité :** régie par le catalogue de codecs ; l'ensemble de base obligatoire (`identity`/`gzip`/`zstd`) garantit une base de référence que n'importe quelle époque peut décoder.

<a id="18-security-considerations"></a>

## 18. Considérations relatives à la sécurité

- La chaîne id/prev assure l'intégrité, et **non** la confidentialité ; utilisez des codecs de classe `encrypt` pour la confidentialité.

- La **troncature** (suppression des trames de fin) est indétectable à partir de la chaîne seule ; un artefact `evidence` DOIT (MUST) ancrer la tête — une signature sur la tête `"id"`, ou la racine de l'index `"head"`/`"mmr"` (§6.2) — afin qu'un vérificateur puisse détecter un journal écourté.

- La **récupération** de trames *après* une trame endommagée n'est garantie qu'avec des décalages (offsets) connus (un index intact, une trame de point de contrôle, ou un cadrage externe) ; une séquence CBOR brute peut se désynchroniser en cas de corruption arbitraire (§9.1). GTS ne définit aucun codage de parité ou d'effacement — la durabilité contre la perte massive relève de la couche de stockage.

- Les valeurs `"to"`/`kid` peuvent laisser fuir des métadonnées de relation (pour qui une trame est scellée). Le profil `opaque` EXIGE (REQUIRES) donc des `kid` pseudonymes ; les autres profils à haute confidentialité DEVRAIENT (SHOULD) les utiliser. Utilisez un identifiant par document ou par paire — par ex. `"kid": "anon:<BLAKE3(true-kid ∥ head-id)>"` — ou l'aveuglement de clé (key blinding), afin que le même destinataire ne soit pas associable d'un fichier à l'autre.

- Une signature valide atteste du signataire sur les octets de la trame ; elle n'affirme **pas** la véracité des affirmations (conformément à la sémantique d'attestation — porter garant ≠ exactitude).

- Les trames opaques sont illisibles mais **non** invisibles ; ne placez pas de secrets dans `"pub"`, `"to"`, ou `"meta"`.

- La compaction par instantané (snapshot) (§10) détruit les signatures originales ; un artefact `evidence` NE DOIT PAS (MUST NOT) faire l'objet d'une compaction par instantané. La compaction diffusable en continu (§10.1) détache les signatures de trame plutôt que de les détruire, mais la chaîne réordonnée n'est attestée que par le compacteur ; un artefact `evidence` NE DOIT PAS (MUST NOT) faire l'objet d'une compaction diffusable en continu, sauf en scellant l'original textuellement (verbatim) (§10.1), et la confiance d'un consommateur dans l'**ordonnancement** d'un fichier compacté est une confiance envers le compacteur.

- La décompression de trames fournies par un attaquant DOIT (MUST) être limitée par le streaming, la contre-pression, la politique de stockage ou l'échec d'allocation de la plateforme, mais les lecteurs NE DOIVENT PAS (MUST NOT) rejeter une trame uniquement parce que sa charge utile décodée dépasse un plafond fixe d'octets au niveau du codec.

- Le GTS imbriqué (§12.1) DOIT (MUST) être limité : les lecteurs DOIVENT (MUST) appliquer une profondeur de récursion maximale et un budget de taille totale décodée pour tous les niveaux d'imbrication (résistance aux bombes matriochka).

- **Les segments sont authentiques de manière indépendante et ne se garantissent pas mutuellement.** La concaténation n'implique aucune approbation : le signataire du segment A n'atteste rien sur le segment B. Un vérificateur DOIT (MUST) signaler les ensembles de signataires par segment (§14.1), et un consommateur décidant de la confiance NE DOIT PAS (MUST NOT) traiter l'union au niveau du fichier comme portant l'autorité du segment le plus fort. La suppression inter-segments par valeur (§11) signifie qu'un segment ajouté non fiable peut CACHER du contenu antérieur à la résolution par défaut — les lecteurs DEVRAIENT (SHOULD) indiquer quel segment a supprimé quoi, et les consommateurs à haute assurance PEUVENT (MAY) ne résoudre la suppression qu'à partir de segments dont ils font confiance aux signataires.

- Un ajout tronqué (torn append) à une limite de segment ressemble à un en-tête tronqué : la règle d'ajout tronqué du §3 s'applique ; les segments précédents se replient (fold) intacts.

<a id="19-conformance-test-vectors"></a>

## 19. Vecteurs de test de conformité

Une mise en œuvre conforme DOIT (MUST) réussir un corpus partagé. La v1 exige au moins ces vecteurs (livrés avec la mise en œuvre de référence), chacun constitué des octets GTS ainsi que du graphe replié attendu (N-Quads) et des diagnostics attendus :

Le document d'accompagnement [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md) définit les déclarations de conformité étagées, les sous-ensembles de vecteurs nommés, les champs JSON attendus, le schéma du manifeste de vecteurs, le registre des diagnostics, ainsi que les modes de lecture/vérification utilisés pour transformer ce corpus en déclarations de mise en œuvre comparables.

1. Fichier valide minimal (en-tête + un `terms` + un `quads`).

2. Une trame `quads` transformée par `zstd`.

3. Une trame à codec inconnu → `reason:"unknown-codec"` opaque.

4. Une trame avec un auto-`"id"` erroné → `DamagedFrame` opaque.

5. Un ajout tronqué à la fin du fichier (EOF) → `TornAppendError`, survivants intacts.

6. Vérification de l'auto-hachage de l'en-tête (positive et altérée).

7. Réificateur RDF 1.2 + aller-retour `annot` (`gts → nq → gts`), incluant la citation sans assertion.

8. Un blob GTS imbriqué (`mt: application/vnd.blackcat.gts+cbor-seq`), récursif et replié.

9. Suppression sur un identifiant de terme (term-id) et sur un condensé (digest) de trame.

10. Détection de troncature par rapport à une racine `"mmr"` d'en-tête/index signée.

11. Définition par défaut du type de données de littéral (§7.1) : un littéral avec `"l"` + `"dir"` et sans `"dt"` → `rdf:dirLangString` ; avec `"l"` et sans `"dt"` → `rdf:langString` ; avec aucun des deux → `xsd:string`.

12. Un réificateur relié à un triplet différent → `ConflictingReifier`, premier lien conservé (§7.8).

13. Une violation de contrainte de position, par ex. un littéral en position de prédicat → rejeté/diagnostiqué (§7.4).

14. Localité des étiquettes de nœuds vides (§7.1, §12.1) : des étiquettes de nœuds vides (bnode) identiques dans un GTS externe et un GTS imbriqué restent isolées (non fusionnées).

15. **Union de deux segments (§3.1)** : le `cat` de deux fichiers à segment unique se replie en l'union de valeurs des deux graphes ; les identifiants de termes (term-ids) sont résolus localement au segment (un IRI partagé s'unifie ; des identifiants identiques nommant des valeurs différentes n'entrent pas en collision) ; les étiquettes de nœuds vides identiques à travers les segments restent isolées.
    *15b* : les nœuds vides sans étiquette (absents **ou vides** `"v"`) sont des termes distincts au sein d'un segment et à travers les segments, et les étiquettes sérialisées de l'union DOIVENT (MUST) les maintenir distincts — un réétiquetage qui fusionne ce que le graphe sépare est le résultat interdit.

16. **Aller-retour composé (§3.1, §14)** : un fichier composé avec `cat` survit à `gts → nq → gts` avec le même repli d'union.

17. **Échec critique du lecteur pré-segment (§17, négatif)** : une implémentation en mode pré-§3.1 alimentée par un fichier à deux segments DOIT (MUST) signaler un diagnostic fatal au second en-tête — le repli de trames au-delà de la limite avec des identifiants de termes globaux au fichier est le résultat interdit que ce vecteur est destiné à capturer.

18. **Suppression entre segments (§11)** : un second segment supprime (a) une trame d'un segment précédent par condensé et (b) un quad par valeur ; la résolution par défaut masque les deux ; les octets du segment supprimé sont vérifiés intacts ; le vérificateur indique quel segment a supprimé quoi (§18).

19. **Union de profils + opacité gracieuse de segment (§3.1)** : un fichier à deux segments dont le second segment nécessite une capacité non déclarée au lecteur replie entièrement le segment un et replie le segment deux sous forme de nœuds opaques avec le profil nommé dans les diagnostics.

20. **Discipline des étiquettes de langue (§13.1, négatif)** : un rédacteur émettant une étiquette de langue à usage privé dans une section de projection/docs DOIT (MUST) échouer au moment de l'écriture ; la même étiquette dans une section de charge utile (payload) `dist` canonique est acceptée.

21. **Composition dégénérée refusée (§14.1, négatif)** : `gts cat` refuse un segment au repli vide et une composition qui supprime tout ; une `cat` d'octets bruts des mêmes entrées produit toujours un fichier structurellement valide (l'outil est plus strict que le format, par conception).

22. **Blob en ligne (§12, §14.1)** : un blob en ligne se replie vers son condensé `blake3:<hex>` avec les métadonnées déclarées (`pub.mt`) conservées ; l'extraction par condensé revérifie les octets ; un blob supprimé par condensé est refusé par défaut.

23. **Propriété de repli de préfixe en continu (§3.2, dérivée)** : ceci n'est pas un vecteur mais un test de propriété sur CHAQUE vecteur de ce corpus — chaque préfixe à la limite d'un élément se replie sans erreur, et à travers les préfixes croissants, les tables repliées ne font que s'étendre (les termes/quads sont des préfixes de liste tant que le nombre de segments est inchangé ; les lignes N-Quads de base (sans nœud vide) sont monotones à travers le passage de la représentation à segment unique à multi-segments).

24. **Compaction diffusable en continu (§3.3, §10.1, §13.3)** : une source accrétive (blobs entrelacés avant leur catalogue, une trame signée par COSE, aucune revendication) et sa réécriture compactée — la réécriture revendique `"layout": "streamable"`, commence par l'index de diffusion en continu, ordonne les blobs du plus significatif au moins significatif, se termine par le pied de page de décalage `index` et porte la provenance de la compaction, incluant la signature de source détachée ; les deux fichiers se replient vers le même graphe de contenu ; les octets compactés sont **gelés** et servent d'oracle de déterminisme inter-moteur (même entrée + même paramètre d'horodatage ⇒ sortie identique au niveau de l'octet dans chaque moteur).

25. **Revendication de diffusion en continu mensongère (§3.3, négatif)** : un segment revendiquant `"layout": "streamable"` qui livre un blob couvert avant les quads décrivant son condensé → `StreamableLayoutError` ; un outil de vérification DOIT (MUST) refuser (quitter avec un code non nul).

26. **Limite d'ajout après compaction (§3.3)** : un segment compacté avec des trames ajoutées après son pied de page `index` se replie proprement sans diagnostic, et l'outillage rapporte « diffusable en continu jusqu'à la trame *N*, queue accrétive » — la queue non annoncée est légale.

27. **Régressions d'hostilité totale du lecteur (§2.4, négatif)** : une entrée vide, un premier élément CBOR qui n'est pas un en-tête (Header), une version majeure d'en-tête non prise en charge, un type de trame structurelle inconnu, une référence de terme vers l'avant et une charge utile de transformation malformée renvoient tous des diagnostics structurés/nœuds opaques le cas échéant. Ces vecteurs figent l'invariant « ne jamais paniquer sur les octets d'entrée » et rendent visible dans l'IC la dérive diagnostique entre les moteurs.

28. **Rédacteur de graphe déterministe (§14.1)** : deux états de graphe repliés équivalents avec des identifiants de termes locaux et un ordre de lignes différents produisent des GTS identiques au niveau de l'octet via une création de graphe déterministe. Le vecteur gelé fige le remappage des termes, le tri des lignes, la rétention des métadonnées de blob, la sortie des métadonnées et le remappage des cibles de suppression à travers les producteurs Python et Rust.

<a id="20-iana-considerations"></a>

## 20. Considérations relatives à l'IANA

Cette section enregistre un type de média. Elle suit les procédures d'enregistrement de
[RFC 6838] et les procédures de suffixe de syntaxe structurée de [RFC 9277]. En attendant
l'enregistrement formel, le type réside dans l'arborescence du fournisseur (`vnd.`) et est utilisé de façon provisoire.

<a id="201-media-type-registration-applicationvndblackcatgtscbor-seq"></a>

### 20.1 Enregistrement du type de média : `application/vnd.blackcat.gts+cbor-seq`

- **Nom du type :** `application`

- **Nom du sous-type :** `vnd.blackcat.gts+cbor-seq`

- **Paramètres obligatoires :** aucun

- **Paramètres facultatifs :** aucun

- **Considérations relatives à l'encodage :** binaire. Un fichier GTS est une séquence CBOR (CBOR Sequence) ([RFC 8742]) et n'est pas limité au texte 7 bits ou 8 bits ; les transports qui ne sont pas « 8-bit clean » DOIVENT (MUST) appliquer un encodage de transfert de contenu (p. ex. base64).

- **Considérations relatives à la sécurité :** voir le §18 de cette spécification. En résumé : la chaîne d'identifiants de contenu (content-id) assure l'intégrité mais pas la confidentialité ; la troncature est indétectable sans engagement de tête (head commitment) ; la décompression et la récursion GTS imbriquée DOIVENT (MUST) être limitées ; et les signatures attestent d'un signataire sur des octets, et non de la véracité des affirmations.

- **Considérations relatives à l'interopérabilité :** le suffixe de syntaxe structurée `+cbor-seq` ([RFC 8742]) signale que la charge utile (payload) est une séquence CBOR (CBOR Sequence), de sorte que les outils de séquence génériques peuvent inspecter les éléments de données ordonnés avant d'appliquer les règles spécifiques à GTS. La balise d'auto-description (self-describe tag) `55799` ([RFC 8949] §3.4.6) PEUT (MAY) marquer chaque en-tête de segment comme un nombre magique. La conformité est définie par le corpus de conformité des vecteurs de test partagé (§19).

- **Spécification publiée :** ce document (GTS — Graph Transport Substrate — Spécification).

- **Applications utilisant ce type de média :** transport et archivage de graphes RDF 1.2 adressés par le contenu ; artefacts de provenance et de mémoire d'agent signés ; distribution de paquets où la charge utile regroupe un graphe et les binaires qu'il référence.

- **Considérations relatives à l'identifiant de fragment :** aucune.

- **Informations supplémentaires :**

  - **Nombre(s) magique(s) :** `0xd9 0xd9 0xf7` (la balise d'auto-description CBOR `55799`) lorsqu'elle est présente au début du fichier (§16.1). Ce préfixe est FACULTATIF (OPTIONAL) car le premier en-tête de segment PEUT (MAY) ne pas être balisé.

  - **Extension(s) de fichier :** `.gts`

  - **Code(s) de type de fichier Macintosh :** aucun

- **Personne et adresse courriel à contacter pour plus d'informations :**
  Patrick Audley <paudley@blackcatinformatics.ca>

- **Usage prévu :** COMMON

- **Restrictions d'utilisation :** aucune

- **Auteur / Contrôleur de changement :** Blackcat Informatics® Inc.

<a id="21-complete-cddl-appendix"></a>

## 21. Annexe CDDL complète

Cette annexe constitue la surface de schéma copiable pour les responsables de la mise en œuvre. Les fragments CDDL intégrés plus haut dans ce document expliquent le contexte local ; cette annexe rassemble les formes de map au niveau de la transmission en un seul endroit.

<a id="211-sequence-grammar"></a>

### 21.1 Grammaire de séquence

Un fichier GTS est une séquence CBOR (CBOR Sequence), et non un seul élément CBOR englobant. Le CDDL décrit les éléments individuels de cette séquence ; la grammaire de la séquence est définie en anglais et en notation de type ABNF :

```text
gts-file = 1*segment
segment  = [ self-describe-tag ] header *frame
```

`self-describe-tag` est l'étiquette CBOR 55799 appliquée à l'élément Header uniquement. Il s'agit d'un indice magique au niveau du câble (wire-level), pas d'un membre de la map Header, et il ne fait pas partie de l'image d'origine (preimage) `"id"` du Header (§22).
Chaque segment commence par un Header, suivi de zéro ou plusieurs éléments de trame (frame) jusqu'au Header suivant ou à la fin du fichier (EOF) (§3.1).

<a id="212-copyable-cddl"></a>

### 21.2 CDDL copiable

```cddl
; GTS v1 item grammar. The top-level file is a CBOR Sequence (§21.1).

gts-item = header-item / frame
header-item = header / self-described-header
self-described-header = #6.55799(header)

term-id = uint
frame-index = uint
codec-id = uint
digest = bstr .size 32
content-id = digest
blake3-uri = tstr                  ; "blake3:" + 64 lowercase hex characters
digest-ref = digest / blake3-uri
profile-name = tstr
layout-state = "streamable" / tstr
extension-key = tstr               ; any text key not defined by that map shape

header = {
  "gts": "GTS1",
  "v": 1,
  "prof": profile-name,
  "cat": { * codec-id => codec },
  ? "layout": layout-state,
  ? "dct": { * tstr => bstr },
  ? "meta": any,
  "id": content-id,
  * extension-key => any,
}

codec = {
  "name": tstr,
  "cls": "encode" / "compress" / "encrypt",
  ? "dct": tstr,
  ? "p": any,
  * extension-key => any,
}

frame = {
  "t": frame-type,
  ? "x": [+ codec-id],
  ? "pub": any,
  ? "to": [+ recipient],
  ? "d": frame-payload / bstr,
  "prev": content-id,
  "id": content-id,
  ? "sig": cose-sign1,
  * extension-key => any,
}

frame-type = "terms" / "quads" / "reifies" / "annot" / "blob" / "suppress"
/ "snapshot" / "meta" / "index" / "opaque"

recipient = {
  "kid": tstr,
  ? "alg": tstr,
  * extension-key => any,
}

cose-sign1 = bstr                  ; serialized COSE_Sign1, detached payload = frame "id"

frame-payload = terms-payload / quads-payload / reifies-payload / annot-payload
/ blob-payload / suppress-payload / snapshot-payload / meta-payload
/ index-payload / opaque-node

terms-payload = [+ term]
term = {
  "k": 0 / 1 / 2 / 3,              ; 0=IRI, 1=literal, 2=bnode, 3=quoted triple
  ? "v": tstr,
  ? "dt": term-id,
  ? "l": tstr,
  ? "dir": "ltr" / "rtl",          ; RDF 1.2 base direction for language-tagged literals
  ? "rf": term-id,
  * extension-key => any,
}

triple-row = [term-id, term-id, term-id]
quad-row = [term-id, term-id, term-id] / [term-id, term-id, term-id, term-id]
reifier-row = [term-id, term-id, term-id, term-id] / [term-id, term-id, term-id, term-id, term-id]
annot-row = [term-id, term-id, term-id] / [term-id, term-id, term-id, term-id]

quads-payload = [+ quad-row]
reifies-payload = [+ reifier-row]
annot-payload = [+ annot-row]

blob-payload = bstr
blob-pub = {
  ? "mt": tstr,
  ? "rep": tstr,
  ? "digest": digest-ref,
  * extension-key => any,
}

suppress-payload = {
  "targets": [+ suppress-target],
  ? "reason": tstr,
  ? "by": term-id,
  * extension-key => any,
}

suppress-target = suppress-frame / suppress-blob / suppress-term
/ suppress-quad / suppress-reifier
suppress-frame = { "kind": "frame", "id": digest-ref, * extension-key => any }
suppress-blob = { "kind": "blob", "digest": digest-ref, * extension-key => any }
suppress-term = { "kind": "term", "id": term-id, * extension-key => any }
suppress-quad = { "kind": "quad", "q": quad-row, * extension-key => any }
suppress-reifier = { "kind": "reifier", "id": term-id, * extension-key => any }

snapshot-payload = {
  "terms": terms-payload,
  ? "quads": quads-payload,
  ? "reifies": reifies-payload,
  ? "annot": annot-payload,
  ? "blobs": { * digest-ref => bstr },
  ? "meta": any,
  * extension-key => any,
}

meta-payload = any

index-payload = {
  "count": uint,
  "head": content-id,
  ? "off": [+ uint],
  ? "ti": { * frame-type => [+ frame-index] },
  ? "dict": [+ frame-index],
  ? "mmr": content-id,
  * extension-key => any,
}

opaque-node = {
  "id": content-id,
  "type": frame-type,
  ? "pub": any,
  ? "to": [+ recipient],
  ? "sigstat": sig-status,
  "reason": opaque-reason,
  * extension-key => any,
}

sig-status = "none" / "valid" / "invalid" / "unverified"
opaque-reason = "unknown-codec" / "missing-key" / "damaged"
/ "unknown-frame-type"

diagnostic = {
  "code": diagnostic-code,
  "detail": tstr,
  ? "frame_index": frame-index,
  * extension-key => any,
}

diagnostic-code = "EmptyFile"
/ "TornAppendError" / "DamagedFrame" / "BrokenChain"
/ "TruncatedLog" / "UnknownCodec" / "MissingKey"
/ "KeyWrapFailed" / "ConflictingReifier" / "IllTypedLiteral"
/ "RecursionLimit" / "StreamableLayoutError" / "PositionConstraint"
/ "ForwardReference" / "SegmentBoundary" / "IndexMmrError"
/ "UnknownFrameType" / tstr

profile-status = "core-required" / "optional-standard" / "experimental"
/ "domain-specific"
profile-registration = {
  "name": profile-name,
  "status": profile-status,
  ? "owner": tstr,
  ? "spec": tstr,
  ? "namespace": [+ tstr],
  ? "requires": any,
  ? "validation": any,
  ? "security": any,
  * extension-key => any,
}
```

Lorsque `"x"` est présent et non vide, la valeur `"d"` de la trame est une chaîne d'octets transportant la charge utile encodée/compressée/chiffrée. Après avoir inversé la chaîne de transformation (§6.1), ces octets se décodent en la charge utile spécifique au type de trame ci-dessus, à l'exception de `blob`, dont la charge utile décodée est constituée d'octets bruts. Lorsque `"x"` est absent, `"d"` transporte directement la charge utile spécifique au type de trame.

`blob-pub` est la forme conventionnelle de la carte `"pub"` d'une trame de blob ; l'enveloppe de la trame maintient `"pub"` typé comme `any` afin que les profils puissent superposer des métadonnées publiques supplémentaires sans modifier la grammaire de base de la trame. `digest-ref` accepte à la fois le condensé brut de 32 octets et la forme textuelle `blake3:<hex>` utilisée par les moteurs de référence.

<a id="22-hash-signature-and-extension-key-preimages"></a>

## 22. Préimages de hachage, de signature et de clé d’extension

Toutes les préimages de cette section utilisent les règles CBOR déterministes du §4 : longueurs définies, entiers de forme la plus courte et clés de carte triées par octet selon leur forme CBOR encodée. À moins qu’une rangée n’exclue explicitement un champ, chaque paire clé/valeur de la carte participe, y compris les clés d’extension inconnues.

<a id="221-preimage-and-subject-table"></a>

### 22.1 Table des préimages et des sujets

| sujet | octets hachés ou signés | champs exclus | champs d'extension inclus | comportement du vérificateur |

|---|---|---|---|---|

| En-tête `"id"` | `BLAKE3-256(deterministic-CBOR(header-map without "id"))` | `"id"` seulement. L'étiquette d'auto-description CBOR 55799 facultative se trouve à l'extérieur de la carte Header et à l'extérieur de la préimage. | Toutes les clés Header inconnues participent. | Recalculer avant d'accepter l'En-tête du segment; une non-correspondance constitue une altération de l'en-tête. |

| Trame `"id"` | `BLAKE3-256(deterministic-CBOR(frame-map without "id" and "sig"))` | `"id"` et `"sig"` seulement. | Toutes les clés de trame inconnues participent. | Recalculer pour chaque trame; une non-correspondance est `DamagedFrame`. |

| Lien de trame `"prev"` | La valeur `"prev"` est incluse dans la préimage de la trame `"id"`. | Aucune au-delà des exclusions de la trame `"id"`. | Les clés de trame inconnues ne modifient pas la sémantique de `"prev"` mais font toujours partie de la préimage de la trame `"id"`. | Comparer au `"id"` de l'élément précédent au sein du même segment; une non-correspondance est `BrokenChain`. |

| Signature de trame COSE | COSE_Sign1 détachée sur les octets de la trame `"id"`. La Sig_structure COSE est `["Signature1", protected, h'', frame-id]` ; le champ payload COSE est `null`/détaché. | La signature ne fait pas partie de la préimage de la trame `"id"` car `"sig"` y est exclu. | Les clés d'extension affectent la signature indirectement en modifiant la trame `"id"`. | Vérifier avec la clé résolue par `kid`; signaler `valid`, `invalid` ou `unverified`. |

| Condensé de blob en ligne | `BLAKE3-256(decoded blob bytes)`, après inversion des transformations et déchiffrement lorsqu'ils sont disponibles. | Les champs d'enveloppe de la trame ne font pas partie du condensé de blob. | Les clés d'extension publiques du blob n'affectent pas le condensé de blob, mais elles affectent la trame contenante `"id"`. | Comparer avec `pub.digest` lorsqu'il est présent et avec les références de graphe qui nomment le blob. |

| Condensé de blob externe | `pub.digest` nomme des octets stockés ailleurs; le sujet du condensé est ces octets externes. | Les octets externes sont absents de la trame GTS, donc seule la revendication du condensé participe à la trame `"id"` via `"pub"`. | Les métadonnées publiques inconnues participent à la trame `"id"`, pas au condensé de blob externe. | Un vérificateur ne peut effectuer la vérification que lorsqu'il obtient les octets externes. |

| Index `"head"` | Le content-id de la dernière trame couverte, où `"count"` est le nombre de trames couvertes par la charge utile de l'index. | Non applicable. | Les clés de charge utile d'index inconnues participent à la trame d'index `"id"`, pas au sujet `"head"`. | Comparer `"head"` à l'identifiant de la trame couverte; une non-correspondance invalide la revendication d'index/de disposition. |

| Index `"mmr"` | Racine Merkle-Mountain-Range sur les identifiants de trames ordonnés couverts par l'index, en utilisant les préimages feuille/parent/racine au §6.2. | La trame d'index elle-même n'est pas couverte à moins qu'un index ultérieur ne la couvre. | Les clés de charge utile d'index inconnues participent à la trame d'index `"id"`, pas à la racine MMR. | Utiliser comme engagement de région couverte entière et racine de preuve optionnels; une non-correspondance est `IndexMmrError`. |

| Provenance de signature détachée | `stream:sourceFrame` nomme la trame originale `"id"`; `stream:cose` transporte les octets COSE_Sign1 originaux. La signature se vérifie toujours sur l'identifiant de trame original. | Le nouveau `"id"` de la trame réécrite n'est pas l'ancien sujet de signature. | Les termes d'extension du graphe de provenance ne modifient pas le sujet de la signature originale. | Vérifier les signatures transportées par rapport à `stream:sourceFrame`; ne pas les traiter comme des signatures sur la trame compactée. |

<a id="222-unknown-extension-key-behavior"></a>

### 22.2 Comportement des clés d'extension inconnues

Une clé d'extension est une clé de mappage de chaîne de texte non définie par la production CDDL de ce mappage. Les clés réservées définies telles que `"id"`, `"sig"`, `"prev"`, `"t"`, `"d"`, `"x"`, `"pub"` et `"to"` ne sont pas des clés d'extension et NE DOIVENT PAS (MUST NOT) être réaffectées par les profils.

Les lecteurs (Readers) DOIVENT (MUST) inclure les clés d'extension inconnues lors du recalcul des préimages d'en-tête (Header) et de trame (frame). Un lecteur (reader) NE DOIT PAS (MUST NOT) rejeter un en-tête (Header), une trame (frame), un codec, un destinataire (recipient), un terme (term), une charge utile (payload), un noeud opaque (opaque-node), un diagnostic ou un mappage d'enregistrement de profil (profile-registration) uniquement parce qu'il contient une clé d'extension inconnue. Les clés inconnues n'ont pas de sémantique de repli (fold) de base à moins qu'un profil ou une extension pris en charge ne les définisse.

Le comportement de réémission dépend de l'opération :

- Les opérations de préservation des octets telles que le `cat` brut, la copie, la mise en miroir ou le service préservent naturellement les clés inconnues car elles préservent les octets d'origine.

- Un outil qui décode et réémet un en-tête (Header) ou une trame (frame) tout en prétendant préserver le même élément logique DOIT (MUST) copier les clés d'extension inconnues textuellement avant de recalculer les valeurs `"id"`.

- Un outil qui ne peut pas préserver les clés d'extension inconnues DOIT (MUST) traiter l'opération comme une réécriture (re-authoring) avec perte, DOIT (MUST) recalculer les valeurs `"id"` et `"prev"` affectées, et NE DOIT PAS (MUST NOT) prétendre que les signatures de trame (frame) existantes restent attachées aux trames réécrites.

- Un compacteur ou un autre outil de réécriture (re-authoring) PEUT (MAY) préserver les anciennes signatures de trame (frame) uniquement en tant que provenance détachée (§10.1), où l'ancien identifiant de trame reste le sujet de signature explicite.

Puisque les clés d'extension participent aux préimages, les auteurs d'extensions peuvent ajouter des métadonnées à preuve d'altération sans changer la grammaire GTS de base. Ils ne peuvent pas changer la grammaire de l'en-tête/de la trame, les préimages de hachage, les sujets de signature ou les sémantiques de repli (fold) (§2.1, §13).

<a id="23-references"></a>

## 23. Références

<a id="231-normative-references"></a>

### 23.1 Références normatives

- **[RFC 2119]** Bradner, S., « Key words for use in RFCs to Indicate Requirement Levels », BCP 14, mars 1997.

- **[RFC 8174]** Leiba, B., « Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words », BCP 14, mai 2017.

- **[RFC 8949]** Bormann, C. et P. Hoffman, « Concise Binary Object Representation (CBOR) », STD 94, décembre 2020.

- **[RFC 8742]** Bormann, C., « Concise Binary Object Representation (CBOR) Sequences », février 2020.

- **[RFC 9052]** Schaad, J., « CBOR Object Signing and Encryption (COSE): Structures and Process », STD 96, août 2022.

- **[RFC 9053]** Schaad, J., « CBOR Object Signing and Encryption (COSE): Initial Algorithms », août 2022.

- **[RFC 9277]** Bormann, C. et M. Nottingham, « On the Use of Structured Suffixes in Media Types », juin 2022.

- **[RFC 6838]** Freed, N., Klensin, J., et T. Hansen, « Media Type Specifications and Registration Procedures », BCP 13, janvier 2013.

- **[RFC 3339]** Klyne, G. et C. Newman, « Date and Time on the Internet: Timestamps », juillet 2002.

- **[BCP 47]** Phillips, A. et M. Davis, « Tags for Identifying Languages », septembre 2009.

- **[BLAKE3]** O'Connor, J., Aumasson, J-P., Neves, S., et Z. Wilcox-O'Hearn, « BLAKE3: one function, fast everywhere » (sortie de 256 bits utilisée ici).

- **[RDF 1.2]** W3C, « RDF 1.2 Concepts and Abstract Data Model », Candidate Recommendation Snapshot, 07 avril 2026, <https://www.w3.org/TR/2026/CR-rdf12-concepts-20260407/> — les termes RDF, le modèle de jeu de données, le terme triplet et le substrat rdf:reifies importés par le §7.

<a id="232-informative-references"></a>

### 23.2 Références informatives

- **[RFC 7049]** Bormann, C. et P. Hoffman, « Concise Binary Object Representation (CBOR) », octobre 2013 (obsolète par le [RFC 8949] ; cité uniquement pour son ordonnancement « canonique » hérité avec la longueur en premier, §4).

- **[RFC 8610]** Birkholz, H., Vigano, C., et C. Bormann, « Concise Data Definition Language (CDDL) », juin 2019.

- **[RFC 9111]** Fielding, R., Nottingham, M., et J. Reschke, « HTTP Caching », juin 2022 (les directives de mise en cache du §16.4).

- **[RFC 6906]** Wilde, E., « The 'profile' Link Relation Type », mars 2013 (l'extension future Accept-Profile notée au §16.4).

---

*GTS est intentionnellement un format de transport, pas une ontologie ou un magasin de graphes. Une mise en œuvre conforme préserve le repli à ajout uniquement et adressé par le contenu afin que des projections indépendantes puissent être régénérées à partir des mêmes octets.*
