<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-PAPER-DRAFT.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# GTS : un substrat de transport adressé par le contenu et en ajout seulement pour les graphes RDF et les artefacts binaires

> Traduction informative de [`docs/GTS-PAPER-DRAFT.md`](../../../../docs/GTS-PAPER-DRAFT.md). Le document anglais demeure la source normative pour les intégrations, les fonctionnalités avancées, les profils optionnels, les données de référence, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md) et reste informative.


Récit de l'ébauche d'article pour le Graph Transport Substrate (GTS).

Ce document est un matériel de recherche informatif. Il ne définit pas le comportement normatif de GTS.
Les exigences normatives demeurent dans [`GTS-SPEC.md`](./GTS-SPEC.md), avec les règles de paliers et de vecteurs testables dans [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md), la politique de confiance/profil dans [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md), et le contrôle des changements dans [`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md).
## Résumé

Les jeux de données RDF sont couramment échangés par le biais de sérialisations textuelles, d'exportations de bases de données, d'archives ad hoc et de formats de paquets spécifiques aux applications. Ces mécanismes sont utiles, mais ils ne fournissent pas un petit substrat commun pour l'historique de graphes en ajout uniquement, les charges utiles binaires adressées par le contenu, la lisibilité partielle et la conformité multi-langage. GTS comble cette lacune en encodant l'état de graphe RDF 1.2 et les actifs binaires référencés sous la forme d'une CBOR Sequence de segments et de trames déterministes. Chaque segment est replié dans un état de graphe, les trames sont liées par des identifiants de contenu BLAKE3, et les charges utiles non prises en charge ou inaccessibles se dégradent en noeuds opaques plutôt que de disparaître. Le dépôt actuel héberge six moteurs de référence en Rust, Python, Go, TypeScript, Smalltalk/Pharo et Kotlin/JVM, ainsi qu'un manifeste de vecteurs et un corpus de conformité partagés utilisés pour comparer les résultats de repli et les diagnostics. GTS n'est pas une base de données, un raisonneur, une ontologie ou un protocole de consensus ; c'est une taille étroite pour le transport de graphes durable et vérifiable.
## 1. Introduction

Les données de graphe traversent désormais les applications local-first, les flux de travail de provenance, les ensembles de preuves, les archives, les limites de service et les systèmes de mémoire d'IA. Un artefact de transport pour ces contextes doit déplacer plus que des triplets : il doit préserver les charges utiles binaires, ajouter l'historique sans réécrire les octets plus anciens, survivre aux codecs ou clés manquants et permettre à des implémentations indépendantes de s'accorder sur la signification des octets.

GTS définit ce problème comme un transport plutôt qu'un stockage. L'artefact durable est un fichier `.gts` servi comme `application/vnd.blackcat.gts+cbor-seq` : une séquence CBOR dont le jeu de données logique est produit par un repli déterministe. Les systèmes de requête, les bases de données, les magasins d'objets, les caches et les profils de domaine se situent autour de l'artefact, et non à l'intérieur du format de base.

Les contributions prévues de ce travail sont :

1. Un format filaire de séquence CBOR pour les journaux de graphes et de binaires en ajout uniquement.
2. Un modèle de repli déterministe pour les termes RDF 1.2, les quads, les réificateurs, les annotations, les blobs, les métadonnées, la suppression, les instantanés et les trames opaques.
3. Une chaîne id/prev adressée par le contenu avec des signatures COSE facultatives, un chiffrement facultatif et un modèle d'opacité pour les capacités manquantes.
4. Une composition multi-segment par concaténation d'octets plus une compaction de disposition diffusable en continu pour les artefacts axés sur la livraison.
5. Un corpus de conformité multi-langues et des implémentations de référence en Rust, Python, Go, TypeScript, Smalltalk/Pharo et Kotlin/JVM.
```text
Applications and profiles
generic graphs | files | evidence | images | media packages | GMEOW | agent memory
|
v
GTS narrow waist
CBOR Sequence segments
deterministic-CBOR headers and frames
BLAKE3 id/prev chains
transform catalog
deterministic fold
opaque-node degradation
|
v
Storage and transport
filesystem | HTTP range | object storage | artifact registries | message buses
```

Le format de base ne s'engage envers aucune ontologie, base de données, moteur de requête, modèle de transaction mutable ou cadre de confiance. Les profils de domaine ajoutent du vocabulaire et de la validation au-dessus de la taille (waist). Les déploiements choisissent le comportement de stockage et de service en dessous de celle-ci. Aucune des deux parties ne modifie la grammaire d'en-tête/trame (frame) de base, les préimages d'identifiant de contenu (content-id), les règles de limite de segment (segment) ou la sémantique de repli (fold).

La famille de packages actuelle est nommée `gmeow-gts` ; le format est GTS. GMEOW est un consommateur en aval et un cas d'utilisation de distribution principal, mais la direction de la dépendance est unidirectionnelle : un lecteur (reader) GTS n'a pas besoin du vocabulaire GMEOW, du raisonnement OWL, des règles du domaine musical ou des conventions de mémoire d'agent pour analyser, vérifier, replier (fold) ou transporter un fichier GTS.
## 3. Format filaire

Un fichier GTS est une CBOR Sequence d'un ou plusieurs segments. Un segment contient un en-tête CBOR déterministe suivi de trames CBOR déterministes. Le type de média provisoire enregistré utilisé par les artefacts publiés est `application/vnd.blackcat.gts+cbor-seq`, et l'extension de fichier est `.gts`.

Au niveau narratif, la structure du fichier est :

```text
GTS file
  segment 0
    header: magic/version/profile/catalog/layout/metadata/id
    frame:  type + transform chain + public envelope + payload + prev + id + optional sig
    frame:  ...
  segment 1
    header
    frame
    ...
```

L'identifiant de contenu de chaque trame est un condensé BLAKE3-256 sur des octets déterministes. Le champ `prev` lie la trame à l'élément précédent dans son segment. Étant donné que les en-têtes de segment et les trames sont des éléments CBOR Sequence, des segments valides indépendants peuvent être concaténés sans réécrire leurs octets. Le fichier résultant se replie comme l'union de valeurs ordonnée des replis de segments.

Les charges utiles utilisent un catalogue de transformations. La surface de base inclut le chemin structurel obligatoire nécessaire pour le lecteur central, tandis que les codecs optionnels et les transformations cryptographiques dépendent des capacités. Les codecs inconnus, les types de trames non pris en charge ou les clés non disponibles sont représentés sous forme de diagnostics et de nœuds opaques du graphe lorsque les octets environnants restent récupérables.

La trame d'index facultative peut transporter des tables de décalage, des index de types de trames et une racine MMR. Le support actuel est intentionnellement limité : la vérification de preuve MMR détachée est inter-moteurs, Rust peut créer des preuves à partir de fichiers GTS indexés, et les surfaces de création de preuves/d'accès aléatoire plus larges restent suivies en tant que primitives avancées plutôt qu'en tant qu'exigences de base du lecteur.
## 4. Sémantique du repli

Le repli (fold) est le rejeu déterministe des trames (frames) de segment en un état ayant la forme d'un dataset RDF.
L'état pertinent inclut :

- Les termes RDF, incluant les IRI, les littéraux, les nœuds blancs (blank nodes) et les triplets cités (quoted triples).
- Les quads et les annotations au niveau des énoncés (statement-level annotations).
- Les liaisons de réificateur (reifier bindings).
- Les sommaires de blobs en ligne par condensé (digest), type de média et taille.
- Les métadonnées de segment, les profils (profiles), les diagnostics et les têtes de segment (segment heads).
- Les enregistrements de suppression et les nœuds opaques (opaque nodes).

Les identifiants de termes (term ids) sont locaux au segment. L'identité inter-segment se fait par valeur de terme RDF, et non par identifiant entier local, et les étiquettes de nœuds blancs (blank-node labels) ne sont pas fusionnées entre les segments produits indépendamment. L'ajout d'un nouveau segment conserve donc les octets existants intacts tout en ajoutant une autre contribution au repli (fold).

La suppression est additive. Elle enregistre une politique d'affichage ou de validité sur les affirmations de graphe antérieures sans supprimer physiquement les anciens octets signés. La compaction d'instantané (snapshot compaction) peut réécrire un graphe en un artefact de distribution plus petit, mais cette réécriture est explicite et entraîne une perte d'information (lossy) par rapport à l'historique complet des ajouts.

L'article devrait traiter le modèle de repli (fold) comme l'abstraction centrale, mais ne devrait pas reformuler de nouvelles règles normatives. La notation formelle peut résumer le modèle de la spécification ainsi :

```text
fold(file) = value_union(fold(segment_0), ..., fold(segment_n))
```

La grammaire exacte, le comportement face aux doublons, le comportement de suppression, les diagnostics et les attentes de conformité (conformance) demeurent du ressort des documents de spécification et de conformité.
## 5. Intégrité, Confidentialité Et Opacité

GTS sépare quatre préoccupations :

- Intégrité des trames (frame integrity) : chaque trame possède son propre identifiant de contenu BLAKE3.
- Intégrité de l'historique (history integrity) : les liens `prev` lient une trame à sa position dans la chaîne.
- Origine ou paternité : des signatures COSE facultatives peuvent lier les signataires aux identifiants de trame.
- Fraîcheur ou non-troncation : un engagement de tête (head commitment) externe ou en bande est nécessaire pour détecter les trames de fin abandonnées.

Les deux premières sont des propriétés de format sans clé. Les deux dernières sont des choix de profil (profile) ou de déploiement. Cette distinction est importante pour le récit de recherche : une signature valide prouve qu'une clé a signé des octets spécifiques, mais la confiance en cette clé et la vérité des affirmations RDF sont des politiques de déploiement ou de profil.

Le modèle d'opacité fait également partie de la conception du transport. Un lecteur (reader) sans codec ou clé peut toujours préserver la position, le type de trame, l'enveloppe publique, les identifiants de destinataire, les signatures et les diagnostics. Le contenu peut être masqué, mais l'existence et la position dans la chaîne du contenu masqué restent observables. Cela rend les lectures dégradées explicites et testables au lieu de supprimer silencieusement des informations.

L'état actuel de la cryptographie v1 devrait être décrit de manière étroite. COSE_Sign1 et COSE_Encrypt0 à destinataire unique sont des capacités de Lecteur Complet (Full Reader) facultatives (optional) implémentées. Les enveloppes COSE_Encrypt multi-destinataires et l'emballage de clé (key-wrap) ECDH sont différés (deferred) en dehors de la conformité v1 jusqu'à ce que des montages (fixtures) au niveau des octets, des tests d'interopérabilité et une politique de gestion des clés existent.
## 6. Statut de conformité et de mise en œuvre

Le dépôt contient six moteurs :

| Moteur | Surface de paquet | Rôle actuel |
|---|---|---|
| Rust | `gmeow-gts`, binaire `gts` | Paquet de référence, API de projection événementielle, création de preuves réservée à Rust, transformations CLI. |
| Python | `gmeow-gts`, module `gts` | Générateur de corpus de référence et paquet Python. |
| Go | `go.blackcatinformatics.ca/gts` | Paquet Go et CLI avec preuve de puits de diffusion (streaming sink). |
| TypeScript | `@blackcatinformatics/gmeow-gts` | Paquet npm, surface de lecteur Node, et surface de flux progressif/WebCrypto pour navigateur. |
| Smalltalk/Pharo | Paquet source Tonel + Metacello, moteur d'exécution Docker `gts` | Moteur Pharo pour le corpus commun, CLI, et surface d'interopérabilité. |
| Kotlin/JVM | Paquet source Gradle et moteur d'exécution `gts` | Moteur JVM pour le corpus commun, CLI, et surface de bibliothèque appelable depuis Java. |

L'oracle de compatibilité partagé est le corpus de vecteurs archivé sous `vectors/` plus les manifestes portables agrégés et délimités sous `vectors/manifest*.json`. Les revendications de conformité nomment un palier (tier), la révision du corpus, les sous-ensembles de vecteurs, les capacités facultatives activées, ainsi que la commande ou le harnais ayant produit la preuve.

Les paliers (tiers) pertinents pour le récit de l'article sont :

- Baseline Reader : analyse, vérification, repli (fold), rapport de diagnostics et dégradation des trames (frames) récupérables non prises en charge en nœuds opaques (opaque nodes).
- Streaming Reader : comportement du Baseline Reader plus une API de puits/événements qui évite de matérialiser l'intégralité du graphe. Dans le dépôt actuel, Go revendique ce palier pour `reader.ReadToSink`, Rust le revendique pour `read_to_sink_from_reader`, et l'exportation vers le navigateur TypeScript le revendique pour `foldStreamToSink`.
- Full Reader : comportement du Baseline Reader plus les capacités facultatives (optional capabilities) revendiquées telles que COSE, le déchiffrement, la récursion GTS imbriquée (nested-GTS), la politique de sécurité ou le comportement d'index/MMR.
- Writer and Validating Tool : sortie déterministe et vérifications d'outils/profils (profiles) plus strictes là où ces revendications sont faites.

Le statut de mise en œuvre devrait être présenté comme un fait évolutif du dépôt, et non comme une revendication de norme. Au moment de cette ébauche, les six moteurs sont décrits comme étant validés par rapport au corpus partagé pour leurs surfaces publiques, tandis que plusieurs capacités restent délibérément en dehors de la base : les exportations vers des bases de données et Parquet ne sont pas présentes dans chaque moteur, la création de preuves non-Rust est différée (deferred), les assistants de récupération par plage (range-fetch) dépendent toujours de limites vérifiées, les modèles de service de stockage d'objets (object-store) sont des contrats d'intégration plutôt qu'un comportement de format de base, et le chiffrement multi-destinataire est épinglé uniquement sous forme de descripteurs de contrat différés (deferred).
## 7. Plan d'évaluation

L'article devrait rapporter des mesures provenant uniquement d'artefacts de version reproductibles. Le dépôt actuel fournit un exécuteur de repère (benchmark) et un modèle de rapport dans [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md). Une exécution de preuves mesurées pour ce projet est conservée en tant que sortie générée dans [`dist/benchmarks/paper-evidence/release-benchmark-report.md`](../../../../dist/benchmarks/paper-evidence/release-benchmark-report.md) plutôt que d'écraser le modèle de repère (benchmark).

L'évaluation devrait couvrir quatre affirmations :

1. Justesse et interopérabilité : chaque moteur replie (folds) les mêmes octets de vecteurs vers les mêmes résumés de graphes, têtes de segments, diagnostics et raisons opaques attendus pour son niveau (tier) revendiqué.
2. Comportement de diffusion en continu : les préfixes de limites d'éléments se replient (fold) vers des états intermédiaires valides, et les moteurs qui revendiquent le statut de lecteur diffusable en continu (Streaming Reader) fournissent des preuves de puits/API avec un comportement de mémoire borné. Les affirmations actuelles de l'article devraient citer Go pour la revendication de niveau (tier) et décrire Rust/TypeScript uniquement comme des preuves événementielles ou progressives.
3. Comportement d'intégrité : les trames (frames) endommagées, les chaînes brisées, les ajouts déchirés, les ancres de troncature et les capacités non prises en charge produisent les diagnostics récupérables ou fatals attendus.
4. Praticité : GTS peut se projeter sur des substrats d'exploitation tels que N-Quads, SQLite, DuckDB et Parquet lorsque le moteur concerné expose ces cibles de transformation, avec les échecs et les lacunes signalés plutôt qu'omis.

Tableaux suggérés pour une annexe de publication :

- succès/échec du corpus par moteur, niveau (tier), sous-ensemble de vecteurs et révision du corpus ;
- temps de lecture (read), de repli (fold), d'écriture (write), d'empaquetage (pack) et de dépaquetage (unpack) par moteur ;
- preuves de mémoire de pointe ou d'allocation pour les chemins de lecteur complet (full-reader) et de lecteur diffusable en continu (streaming-reader) ;
- comparaisons de taille de fichier à travers les choix de codec et le compactage diffusable en continu (streamable compaction) ;
- comportement de récupération sur entrée corrompue avec et sans index de décalage (offset) ;
- économies d'octets par récupération de plage (range-fetch) pour les exemples de livraison progressive une fois les limites connues.
## 8. Applications

Le GTS est conçu pour prendre en charge plusieurs familles d'applications sans faire de l'une d'entre elles l'identité centrale :

- Distribution de jeux de données et d'ontologies : publier un paquet de graphes vérifiable avec les ressources binaires qu'il nomme.
- Distribution GMEOW : expédier des paquets d'ontologies et des profils GMEOW en tant qu'artefacts GTS tout en maintenant le GTS indépendant de GMEOW.
- Archives et manifestes de fichiers : emballer des arborescences de répertoires avec des métadonnées natives aux graphes et des blobs adressés par le contenu.
- Preuves et chaînes de possession : ajouter des observations, des signatures et des charges utiles scellées sans réécrire l'historique antérieur.
- Synchronisation de graphes « local-first » : concaténer des segments produits indépendamment et effectuer le repli de l'union des valeurs.
- Paquets d'images et de médias : commencer par les métadonnées de catalogue et de petites manifestations, puis transporter plus tard des blobs plus volumineux et la provenance dans le même flux vérifiable.
- Mémoire d'agent et révision des croyances : ajouter des observations, des suppressions et la provenance sous forme d'un profil au niveau applicatif plutôt que comme l'identité du format.
- Échange de bases de données de graphes : projeter l'état de graphe replié vers N-Quads, SQLite, DuckDB, Parquet ou d'autres systèmes lorsque ces transformations sont disponibles.
## 9. Limites et travaux futurs

Le GTS n'est pas un langage de requête, un raisonneur, une base de données muable, un protocole de consensus, un système de découverte de clés, un cadre de confiance ou une garantie de disponibilité de blobs externes. La résolution de conflits au niveau applicatif reste au-dessus du repli (fold) central, et les déploiements demeurent responsables des ancres de confiance, de l'autorisation des signataires, de la rotation des clés, de la révocation et des engagements de tête (head commitments) externes.

Les limites connues et les éléments différés (deferrals) actuels incluent :

- La détection de troncature nécessite un engagement de tête (head commitment) tel qu'une tête signée, une racine d'index, un manifeste de version ou une ancre externe.
- Les trames (frames) confidentielles peuvent encore révéler l'existence, le type, les identifiants de destinataires, les signatures et la position dans la chaîne.
- La récupération après une corruption d'octets arbitraire nécessite des décalages (offsets) connus ou un tramage (framing) externe ; une CBOR Sequence brute peut perdre la synchronisation après des octets endommagés.
- La compression, la décompression et la récursion GTS imbriquée nécessitent des budgets de ressources explicites.
- Le COSE_Encrypt multi-destinataire et le ECDH key-wrap sont en dehors de la conformité v1.
- La création de preuves multi-moteurs, les assistants de récupération de plage (range-fetch) plus profonds et les flux de travail (workflows) de magasin d'objets/services sont des surfaces avancées plutôt que des exigences de lecteur (reader) centrales.
- Les profils (profiles) de normes optionnelles et spécifiques au domaine nécessitent une gouvernance, des vecteurs de test et des notes de compatibilité claires avant de pouvoir faire des affirmations fortes.
- Les affirmations de version et de publication nécessitent des révisions de corpus estampillées plutôt que l'espace réservé (placeholder) du manifeste enregistré.
## 10. Travaux connexes

GTS chevauche délibérément plusieurs domaines matures, mais il occupe un point différent dans l'espace de conception : un artefact de transport unique qui est à ajout uniquement, adressé par le contenu, de forme RDF après repli, conscient de la charge utile binaire, partiellement lisible et couvert par un corpus de conformité multi-moteurs.

**Sérialisations RDF et échange de graphes.** Les sérialisations RDF du W3C telles que
[RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/),
[TriG](https://www.w3.org/TR/rdf12-trig/), N-Triples/N-Quads, Turtle et
[JSON-LD 1.1](https://www.w3.org/TR/json-ld11/) définissent des moyens interopérables d'écrire des graphes
ou des ensembles de données RDF. HDT, la soumission de membre du W3C pour
[Header-Dictionary-Triples](https://www.w3.org/submissions/2011/SUBM-HDT-20110330/), traite de la
publication et de l'échange RDF binaire compact. GTS diffère en traitant la projection RDF comme
le repli d'un journal binaire à ajout uniquement qui peut également transporter des blobs
adressés par le contenu, des transformations, des signatures, des diagnostics d'opacité et un
historique multi-segments.

**Encodages binaires, séquences et paquets.** GTS réutilise
[CBOR](https://www.rfc-editor.org/info/rfc8949) pour la structure binaire déterministe et les
[CBOR Sequences](https://datatracker.ietf.org/doc/html/rfc8742) pour les flux d'éléments à
auto-délimitation. Les systèmes d'empaquetage de données d'archivage et de recherche tels que
[BagIt](https://datatracker.ietf.org/doc/rfc8493/) et
[RO-Crate](https://www.researchobject.org/specs/) se concentrent sur le transfert de fichiers
fiable et la description d'objets de recherche riches en métadonnées. GTS emprunte l'intuition du
paquet et du manifeste, mais le manifeste du paquet est lui-même un état de graphe replié et
chaque segment/trame participe à la même chaîne d'identifiants de contenu.

**Systèmes adressés par le contenu et journaux à ajout uniquement.** Git est couramment décrit par sa propre
documentation comme un [content-addressable filesystem](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects),
et IPFS nomme le contenu via des [CIDs](https://docs.ipfs.tech/concepts/content-addressing/) qui
dérivent de hachages cryptographiques. Les systèmes de transparence tels que
[Certificate Transparency](https://datatracker.ietf.org/doc/html/rfc6962) utilisent des journaux à
ajout uniquement et des preuves d'audit pour des événements d'émission observables à l'échelle
mondiale. GTS applique l'adressage par le contenu à l'intérieur d'un artefact de graphe portable : les
identifiants de trame et les liens `prev` confèrent une intégrité de chaîne locale, les index MMR
facultatifs prennent en charge les preuves d'inclusion détachées, et les profils de déploiement décident
s'il faut ancrer les têtes dans un système de transparence ou de publication externe.

**Sourcing d'événements et synchronisation locale d'abord.** Le sourcing d'événements enregistre
les changements d'état comme une séquence d'événements à partir desquels l'état peut être
reconstruit, tel que résumé dans le modèle
[Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) de Martin Fowler. Les travaux sur le
local-first plaident pour des données contrôlées par l'utilisateur et capables de fonctionner hors
ligne, avec la synchronisation comme une amélioration plutôt qu'une dépendance centrale,
notamment dans l'essai [local-first software](https://www.inkandswitch.com/essay/local-first/)
d'Ink & Switch. GTS n'est pas un CRDT ou un protocole de synchronisation d'applications, mais
sa concaténation de segments, sa validité de repli de préfixe et son modèle de suppression
additive le rendent approprié comme artefact de transport pour les systèmes qui ont besoin
d'historiques à ajout uniquement et d'une projection ultérieure dans une logique de fusion
spécifique à l'application.

**Provenance, garde et preuves de recherche.** Le
[PROV-O](https://www.w3.org/TR/prov-o/) du W3C fournit un vocabulaire RDF/OWL pour représenter et
échanger des informations de provenance. RO-Crate et BagIt fournissent des modèles
d'empaquetage établis pour les objets de recherche et les charges utiles de préservation
numérique. GTS peut transporter des métadonnées PROV-O, de type RO-Crate ou spécifiques à un
domaine comme du contenu de graphe ordinaire, tout en séparant l'intégrité des octets et la
vérification des signatures de la confiance de déploiement : une chaîne GTS valide prouve la
continuité des octets, et non la vérité des affirmations ou l'autorité d'un signataire.
**Couches de sécurité de la charge utile.** GTS utilise COSE plutôt que d'inventer une enveloppe de signature ou de chiffrement : la [RFC 9052](https://www.rfc-editor.org/info/rfc9052) définit les structures de signature, de MAC et de chiffrement pour la sérialisation CBOR. Les écosystèmes JSON utilisent couramment [JWS](https://datatracker.ietf.org/doc/html/rfc7515) pour les charges utiles basées sur JSON protégées en intégrité. La distinction de GTS est l'invariant d'opacité : les charges utiles chiffrées ou non prises en charge peuvent rester visibles dans le graphe en tant que nœuds opaques avec des diagnostics, des enveloppes publiques et une position dans la chaîne plutôt que de provoquer un échec de lecture total ou de disparaître du repli.

**Bases de données de graphes et cibles de projection.** SPARQL 1.1 définit le [langage de requête standard pour RDF](https://www.w3.org/TR/sparql11-query/), tandis que des systèmes tels que SQLite, DuckDB et Parquet fournissent des substrats tabulaires durables ou analytiques. SQLite documente un [format de base de données à fichier unique](https://www.sqlite.org/fileformat.html) stable ; DuckDB est une [base de données analytique intégrable](https://duckdb.org/pdf/SIGMOD2019-demo-duckdb.pdf) ; et Apache Parquet est un [format de fichier orienté colonne](https://parquet.apache.org/) pour l'analytique. GTS ne concurrence pas ces systèmes en tant que moteur de requête. Au lieu de cela, il définit un transport portable et vérifiable à partir duquel des N-Quads, SQLite, DuckDB, Parquet ou des magasins RDF natifs peuvent être régénérés.
## 11. Conclusion

GTS explore une mince couche de transport pour les artefacts en forme de graphe : des octets CBOR déterministes,
des trames en ajout uniquement, un historique adressé par le contenu, une sémantique de repli, une opacité gracieuse et un
corpus de conformité interlangue. Sa valeur réside dans la frontière qu'il trace. L'artefact central est
portable et vérifiable ; des bases de données plus riches, des profils, des systèmes de preuve, des magasins d'objets et des flux de travail
de domaine peuvent s'y rattacher au-dessus ou en dessous sans changer la taille de guêpe du format.
## Ébauches d'annexes

Les futures révisions du document peuvent ajouter :

- Des extraits CDDL de la spécification.
- Le pseudocode de repli (fold) aligné sur l'algorithme normatif.
- Un résumé du catalogue des vecteurs de conformité.
- L'extrait de l'enregistrement du type de média.
- Des exemples CLI pour les cibles read, verify, fold, cat, compact, pack, unpack et transform.
- Une liste de contrôle de sécurité résumant les hypothèses d'intégrité, de confiance, d'opacité et de limites de ressources.
