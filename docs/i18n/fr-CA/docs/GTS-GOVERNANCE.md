<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-GOVERNANCE.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Politique de gouvernance et de version de GTS

> Traduction informative de [`docs/GTS-GOVERNANCE.md`](../../../../docs/GTS-GOVERNANCE.md). Le document anglais demeure la source faisant autorité pour la gouvernance, la sécurité, les versions, les licences, la contribution, les obligations de conduite, les processus de divulgation et les commandes exécutables. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

Ce document définit le processus léger de contrôle des changements pour GTS, les politiques du registre d'extension, le contrat de compatibilité et le parcours de la version candidate v1.0. Il complète la spécification du format filaire dans [`GTS-SPEC.md`](./GTS-SPEC.md), la politique de conformité dans [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md), la politique de profil/sécurité dans [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md) et la politique de divulgation de sécurité du dépôt dans [`../SECURITY.md`](../SECURITY.md).

GTS est indépendant de l'ontologie. GMEOW est un consommateur en aval et un cas d'utilisation de distribution principal, mais les lecteurs et rédacteurs GTS ne nécessitent pas le vocabulaire, l'outillage ou la sémantique de GMEOW.

## 1. Classes de changement

GTS utilise différents parcours de gouvernance pour les changements au format de base et les ajouts de profils ou de registres. L'objectif est de maintenir la stabilité de l'étroitesse de la trame du format de transmission (wire-format) tout en permettant aux profils et aux modèles de déploiement d'évoluer.

| classe de changement | exemples | parcours requis |
|---|---|---|
| Changement au format de transmission (wire-format) de base | Grammaire de l'en-tête ou de la trame, règles CBOR déterministes, préimages d'identifiant/hachage, sémantique `prev`, limites de segment, mécanique du catalogue de transformation, sémantique de repli (fold) de base. | Proposition d'amélioration GTS (GIP), PR de spécification, examen de l'impact sur le corpus de conformité et plan d'implémentation multi-moteurs. |
| Changement de conformité du lecteur/rédacteur (reader/writer) | Niveaux de lecteur de base, comportement de diagnostic, attentes des vecteurs, exigences de déterminisme du rédacteur. | GIP lorsque le comportement change ; PR de documentation/corpus pour les clarifications qui préservent le comportement. |
| Profil standard optionnel | `files`, `evidence`, `opaque`, `bundle`, `stream`, ou un nouveau profil destiné à être maintenu par le noyau GTS. | Proposition de profil plus GIP s'il ajoute des obligations standard, une politique de sécurité ou des niveaux de conformité. |
| Profil spécifique à un domaine | Profils de distribution GMEOW, `music-package`, `agent-memory`, et profils d'application tiers. | Examen de l'enregistrement du profil ; pas de GIP de base sauf si le profil nécessite un nouveau comportement de base. |
| Ajout au registre | Nouveau nom de codec, code de diagnostic, type de trame, nom de profil ou cible de transformation. | Politique de registre à la Section 3. |
| Changement d'implémentation ou de paquet | API du moteur, ergonomie du CLI, empaquetage, documentation, exemples. | Flux normal de tickets/PR à moins qu'il ne change le comportement de conformité. |

## 2. Propositions d'amélioration de GTS (GTS Improvement Proposals)

Une proposition d'amélioration de GTS (GIP) est requise avant qu'une modification ne puisse altérer la sémantique de base de GTS ou le comportement requis des implémentations conformes.

### 2.1 Quand un GIP est requis

Ouvrez un GIP pour :

- tout changement à la grammaire d'en-tête ou de trame ;
- tout changement aux préimages de hachage, aux préimages de signature ou aux exigences CBOR déterministes ;
- tout changement à la composition des segments, aux règles de chaîne `prev` ou à la sémantique de repli ;
- l'ajout, la suppression ou la reclassification d'une exigence de Baseline Reader, Writer, Validating Tool ou Full Reader ;
- le fait de rendre obligatoire une capacité facultative ;
- l'ajout d'un profil de norme facultative (optional-standard profile) maintenu par le noyau GTS ;
- la promotion d'un codec, d'un type de trame ou d'un diagnostic dans une entrée de registre requise par le noyau ;
- la modification de la politique de compatibilité pour le format filaire (wire format), le corpus, les paquets ou les profils.

Un GIP n'est pas requis pour les clarifications éditoriales, les exemples, l'enregistrement de profils spécifiques à un domaine, les métadonnées de paquets ou la refactorisation de l'implémentation qui préserve le comportement de conformité observable.

### 2.2 Forme d'une GIP

Une GIP DEVRAIT (SHOULD) être ouverte sous la forme d'un ticket (issue) GitHub ou d'un document de conception intégré avec :

- titre et résumé;
- classe de changement et sections de la spécification (spec) affectées;
- motivation et non-objectifs;
- changement exact du comportement normatif;
- impact sur la compatibilité pour le format filaire (wire), le corpus, les paquets (packages) et les profils (profiles);
- vecteurs de conformité à ajouter ou à mettre à jour;
- moteurs affectés et plan de mise en œuvre;
- considérations relatives à la sécurité et à la vie privée;
- plan de migration et jalon de version (release milestone);
- alternatives envisagées;
- état de la décision : `draft`, `proposed`, `accepted`, `implemented`, `rejected` ou `superseded`.

Les GIP acceptées sont mises en œuvre par le biais de demandes de tirage (pull requests) normales. La PR qui met en œuvre une GIP DOIT (MUST) lier la GIP et mettre à jour la spécification (spec) pertinente, le corpus de conformité (conformance corpus), la documentation et les moteurs.

## 3. Gouvernance des registres

Les tables de registre canoniques peuvent (MAY) résider dans la spécification, le document de conformité, la politique de sécurité ou les futurs fichiers `docs/registries/`. Cette section définit la politique de changement pour chaque registre.

| registre | exemples | politique de changement |
|---|---|---|
| Types de trame | `terms`, `quads`, `blob`, `snapshot`, `index`, types de trame d'extension. | Spécification requise. Les nouveaux types de trame de base ou standards optionnels requièrent un GIP, une mise à jour de la spécification, des vecteurs et un comportement de déclassement pour les lecteurs inconnus. |
| Diagnostics de base | `DamagedFrame`, `UnknownCodec`, `StreamableLayoutError`, `RecursionLimit`. | Spécification/conformité requise. Les ajouts nécessitent une mise à jour du document de conformité et des vecteurs lorsque le diagnostic est observable dans un niveau requis. Les renommages sont des changements majeurs (breaking). |
| Noms de codec | `identity`, `gzip`, `zstd`, `cose-encrypt0`, futurs noms de compression ou de chiffrement. | Examen par des experts ou PR avec preuve d'implémentation. Les codecs de base requièrent un GIP. Les codecs optionnels requièrent un comportement de repli/opaque et des notes d'interopérabilité. |
| Codecs sensibles à la sécurité | Chiffrement, signature, enveloppement de clé (key-wrap), décompression, conteneur imbriqué (nested-container) ou transformation exécutable (executable-transform). | Examen par des experts plus considérations de sécurité, analyse du budget de ressources et vecteurs avant les revendications de conformité. Une promotion obligatoire requiert un GIP. |
| Profils | `files`, `stream`, `evidence`, `opaque`, profils tiers. | Enregistrement selon le principe du premier arrivé avec examen pour les profils spécifiques à un domaine. Les profils standards optionnels ou maintenus par le noyau requièrent un examen plus approfondi et peuvent (MAY) nécessiter un GIP. |
| Cibles de transformation | N-Quads, Turtle, SQLite, DuckDB, Parquet, dispositions d'externalisation de blob (blob externalization layouts). | PR de documentation pour la forme de la cible et les attentes de cycle complet (round-trip). Les cibles CLI/API implémentées mettent également à jour les documents de parité et les tests. |

### 3.1 Espaces de noms réservés

Les noms simples dans les ensembles suivants sont réservés pour le registre central de GTS :

- les types de trame déjà nommés par `GTS-SPEC.md`, ainsi que les futurs noms de trame courts en minuscules acceptés par la politique de types de trame ;
- les codes de diagnostic sans préfixe de profil ou de propriétaire, particulièrement les codes `PascalCase` dans `GTS-CONFORMANCE.md` ;
- les noms de codec dans la table de codec canonique et les futurs noms courts acceptés par la politique de codec ;
- les noms de profils standards : `generic`, `dist`, `evidence`, `opaque`, `bundle`, `files`,
  `stream`, `image`, `ai-package`, et tout futur profil maintenu par le noyau ;
- les IRI appartenant à GTS sous `https://w3id.org/gts/`;
- les noms de cibles de transformation CLI utilisés par ce dépôt, tels que `nquads`, `sqlite`, `duckdb`,
  `parquet`, et `turtle`.

Les enregistrements de tiers DEVRAIENT (SHOULD) utiliser l'un des éléments suivants :

- un URI stable ;
- un nom DNS inversé tel que `org.example.profile` ;
- un jeton préfixé par le propriétaire tel que `example-profile` lorsque le propriétaire est évident d'après la ligne du registre.

Les diagnostics spécifiques à un profil DEVRAIENT (SHOULD) utiliser un espace de noms ou un préfixe de profil documenté. Un profil NE DOIT PAS (MUST NOT) prétendre que ses diagnostics, types de trames ou noms de codec sont des comportements centraux de GTS à moins qu'ils n'aient été acceptés dans le registre central correspondant.

## 4. Gouvernance des profils

Les modifications de la spécification de base et les ajouts de profils suivent des parcours distincts.

Des profils spécifiques à un domaine peuvent être enregistrés sans modifier le format filaire. Une définition de profil DOIT (MUST) stipuler qu'elle ne modifie pas :

- la grammaire de l'en-tête ou de la trame (frame) ;
- la détection des limites de segment (segment) ;
- les pré-images de content-id, de signature ou de hachage ;
- la résolution de transform-catalog ;
- la sémantique de repli (fold) déterministe.

Un profil peut définir un vocabulaire, des règles de validation, une politique de confiance, un flux de travail de publication et des vecteurs de conformité. Un profil peut exiger des signatures, des clés, des codecs ou une confiance de déploiement pour les outils prenant en charge les profils sans rendre ces fonctionnalités obligatoires pour les Baseline Readers.

Le modèle d'enregistrement de profil dans `GTS-SPEC.md` demeure la liste de contrôle de contenu requise pour un nouveau profil. La promotion d'un profil au statut de norme optionnelle (optional-standard) exige en outre :

- un propriétaire nommé et un contrôleur de changement ;
- au moins une implémentation ou un validateur exécutable ;
- des vecteurs de conformité spécifiques au profil ;
- un examen de la sécurité et de la protection de la vie privée ;
- une politique de compatibilité pour les futures révisions du profil.

## 5. Politique de compatibilité

La compatibilité GTS est divisée en quatre couches. Une version ou une proposition DOIT (MUST) nommer la couche qu'elle affecte.

| couche | règle de compatibilité |
|---|---|
| Compatibilité du format filaire (wire-format) | Le champ d'en-tête `"v"` est la version majeure du format filaire. Avant la v1.0, des changements incompatibles restent possibles. Après la v1.0, un fichier qui est une version majeure 1 de GTS valide doit rester analysable et repliable (foldable) en toute sécurité par les futurs lecteurs (readers) de version majeure 1, sous réserve des capacités déclarées et des limites de ressources. Les changements de rupture du format filaire nécessitent une nouvelle version majeure du format filaire. |
| Compatibilité du corpus | Le corpus de vecteurs est l'oracle de compatibilité pour les niveaux (tiers) revendiqués. De nouveaux vecteurs peuvent être ajoutés pour clarifier le comportement ou couvrir des régressions. Les vecteurs existants ne peuvent changer que par le biais d'un GIP ou d'une correction expliquant que l'attente précédente était erronée. Les versions candidates (release candidates) DEVRAIENT (SHOULD) joindre des rapports de conformité nommant le commit du corpus. |
| Compatibilité des paquets | Les paquets Rust, Python, Go et TypeScript sont des artefacts de version. Ils DEVRAIENT (SHOULD) maintenir les API destinées aux utilisateurs stables selon les règles semver normales de leur écosystème. Les versions des paquets peuvent différer de la version du document et de la version du corpus, mais les notes de version DOIVENT (MUST) indiquer le commit de la spécification/du corpus qu'ils implémentent. |
| Compatibilité des profils | Les profils (profiles) sont propriétaires de leur vocabulaire et de leur compatibilité de validation. Les profils spécifiques à un domaine peuvent avoir des versions indépendantes, mais une révision de profil DOIT (MUST) préserver la sémantique de base de GTS pour l'analyse, la vérification et le repli (fold) pour les lecteurs (readers) de référence. Les profils de normes optionnelles nécessitent des notes de compatibilité dans le registre. |

Les revendications de compatibilité DEVRAIENT (SHOULD) identifier :

- le nom de l'implémentation et la version du paquet ;
- la version majeure du format filaire (wire-format) ;
- la version du document ou le commit de la spécification ;
- le commit et le niveau (tier) du corpus ;
- les capacités optionnelles activées ;
- les versions de profil ou les rangées du registre de profils utilisées.

## 6. Gouvernance de la sécurité de l'analyseur et de la cryptographie

Le processus de divulgation au niveau du dépôt est défini dans [`../SECURITY.md`](../SECURITY.md). Cette
section définit comment les changements de format et de registre sensibles à la sécurité sont gouvernés.

Les changements sensibles à la sécurité incluent :

- L'analyse (parsing) CBOR, l'encodage déterministe et la détection de limite de segment ;
- La décompression, les dictionnaires de compression et les limites de bombes de décompression ;
- Le chiffrement, les signatures, les identifiants de clés, l'enveloppement de clé (key-wrap) et le comportement de la politique de confiance ;
- L'extraction de profil `files` ou toute commande qui écrit sur le disque ;
- La gestion de GTS imbriqué, la récursion ou les limites de taille décodée ;
- Les règles de profil qui modifient ce qu'un outil traite comme étant de confiance, publiable ou sûr.

Toute proposition de codec, de transformation ou de profil sensible à la sécurité DOIT (MUST) documenter :

- Le modèle de menace et les non-objectifs ;
- L'impact sur le budget des ressources ;
- Le comportement de déclassement (downgrade) ou opaque lorsqu'il n'est pas pris en charge ;
- Les diagnostics d'échec ;
- Les vecteurs ou les tests pour les entrées hostiles ;
- La coordination de la divulgation et de la version si le changement corrige une vulnérabilité.

Les vulnérabilités confirmées dans l'analyseur, la cryptographie, l'extraction ou les pipelines de version suivent
une divulgation coordonnée privée. Les GIP publics devraient éviter les détails d'exploitation jusqu'à ce que le
correctif ou l'avis soit prêt.

## 7. Chemin de version v1.0

Le chemin v1.0 est échelonné afin que la spécification de base puisse être publiée sans attendre chaque futur profil, transformation ou artefact de recherche.

| jalon | critères de publication |
|---|---|
| `v1.0-alpha1` | Le tramage autonome est en place, la décision sur le type de média est documentée, les annexes CDDL/hash-preimage ont un chemin d'ébauche, et l'indépendance de GMEOW est explicite. Des modifications au format filaire (wire-format) sont encore attendues. |
| `v1.0-beta1` | Le manifeste de conformité ou la politique de corpus équivalente est stable, les politiques de registre existent, la séparation noyau/profil est claire, et la sémantique de repli (fold) est suffisamment formelle pour l'examen des responsables de mise en œuvre. Les modifications au format filaire (wire-format) nécessitent des notes de compatibilité explicites. |
| `v1.0-rc1` | Aucune modification intentionnelle du format filaire (wire-format) ne subsiste, les vecteurs de référence sont gelés, les politiques de registre et les espaces de noms réservés sont publiés, le modèle de sécurité est clair, et l'examen des responsables de mise en œuvre ne présente aucune conclusion bloquante ouverte. |
| `v1.0` | La spécification du format est publiée, le corpus de conformité est étiqueté, les paquets de l'implémentation de référence sont publiés, et les notes de version identifient les commits de la spécification/du corpus. |

Le guide d'exécution concret `v1.0-rc1` est [`GTS-V1-RC1-CHECKLIST.md`](./GTS-V1-RC1-CHECKLIST.md). Il enregistre le commit de la spécification, la révision du corpus, l'examen des éléments bloquants, les rapports de conformité, les essais à blanc (dry-runs) des paquets, les notes de version, les flux de travail d'étiquetage, et les preuves de vérification d'artefacts requises pour une coupure de version candidate.

### 7.1 Bloqueurs pour v1.0-rc1

Les éléments suivants bloquent la publication de `v1.0-rc1` :

- changements intentionnels non résolus apportés à la grammaire d'en-tête/de trame, aux pré-images de hachage, aux pré-images de signature, à la composition de segments, à la résolution de transformations ou à la sémantique de repli (fold) de base ;
- vecteurs de conformité de base manquants ou échoués pour le comportement requis du lecteur/rédacteur ;
- échec inter-moteur pour un comportement revendiqué par le niveau de base v1 ;
- politiques de changement de registre manquantes pour les types de trames, les diagnostics, les codecs, les profils et les cibles de transformation ;
- directives de l'espace de noms réservé manquantes ;
- vulnérabilité non résolue de sévérité élevée ou critique dans l'analyseur (parser), la crypto, l'extraction ou le pipeline de version (release) ;
- directives de type de média et de distribution/HTTP manquantes, nécessaires pour une publication immuable ;
- langage de compatibilité peu clair pour le format filaire (wire format), le corpus, les paquets ou les profils.

### 7.2 Éléments non bloquants pour v1.0-rc1

Les éléments suivants ne bloquent pas `v1.0-rc1` lorsque le niveau de conformité de base est par ailleurs prêt :

- l'achèvement de chaque profil de norme optionnelle ou spécifique à un domaine ;
- l'implémentation de chaque cible de transformation dans tous les moteurs ;
- les outils de base de données, Parquet, navigateur, magasin d'objets, index/MMR, réplication, range-fetch ou de preuve avancée ;
- chaque mode de gestion de clés ou enveloppe de chiffrement multi-destinataire ;
- le renommage de paquets, les alias de paquets neutres ou la soumission à des organismes de normalisation ;
- la publication de paper, de benchmark-suite et de third-party implementation-guide ;
- les futures entrées du registre de profils.

Les éléments non bloquants DEVRAIENT (SHOULD) être suivis en tant qu'issues, éléments de projet ou lignes de liste de contrôle adjacentes à la version, mais ils NE DOIVENT PAS (MUST NOT) retarder la publication de la conformité de base à moins qu'ils n'exposent un élément bloquant ci-dessus.

## 8. Livrables connexes à la version

Ces livrables sont utiles pour l'adoption et la posture relative aux normes, mais ne constituent pas des obstacles à la publication de la spécification de base :

| livrable | attente en matière de suivi | relation avec la version |
|---|---|---|
| Aperçu de l'article et ébauche de publication | Suivre comme un problème de documentation/recherche ou un élément de projet. Ébauche actuelle : [`GTS-PAPER-DRAFT.md`](./GTS-PAPER-DRAFT.md). Réutiliser les résultats autonomes de trame (framing) et de conformité. | Décrit et motive le GTS ; ne définit pas de comportement normatif. |
| Suite de tests de performance (Benchmark) | Suivre les tests de performance (benchmarks) de mémoire, de lecture (read), de repli (fold), d'écriture (write), de compression (pack), de décompression (unpack) et d'interopérabilité entre moteurs. Enregistrer le matériel, le commit de la spécification et le commit du corpus. Modèle de rapport actuel : [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md). | Soutient les affirmations de performance ; ne bloque pas la validité du format filaire (wire-format). |
| Guide d'implémentation par des tiers | Suivre les exemples, les conseils sur les modèles de profil (profile), le processus de registre et le tutoriel du lecteur (reader) minimal ; guide actuel : [`GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md`](./GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md). | Aide les implémenteurs à adopter la spécification v1 ; non requis pour `v1.0-rc1` si les documents de conformité sont suffisants. |

Chaque liste de contrôle de version candidate (release-candidate) v1 DEVRAIT (SHOULD) indiquer si ces livrables sont terminés, différés (deferred) ou assignés à des problèmes de suivi.
