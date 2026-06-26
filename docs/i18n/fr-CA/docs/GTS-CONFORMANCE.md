<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-CONFORMANCE.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Conformité GTS

> Traduction informative de [`docs/GTS-CONFORMANCE.md`](../../../docs/GTS-CONFORMANCE.md). Le document anglais demeure la source normative pour les règles de compatibilité, les déclarations de conformité, les matrices de parité, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

Ce document définit comment une implémentation formule une déclaration de conformité testable pour le Graph Transport Substrate (GTS). Il accompagne le [`GTS-SPEC.md`](./GTS-SPEC.md) : la spécification définit le format filaire et le comportement ; ce document définit les niveaux (tiers), les sous-ensembles de vecteurs, les formats de résultats attendus, les diagnostics et les modes de lecture utilisés pour comparer les implémentations.

## 1. Revendications de conformité

Une déclaration de conformité DOIT (MUST) nommer :

- le nom et la version de l'implémentation ;
- le ou les paliers de conformité revendiqués (§3) ;
- le mode de lecture (read mode) ou le mode de vérification (verify mode) utilisé (§7) ;
- la révision du corpus, généralement le commit du dépôt contenant `vectors/` ;
- les sous-ensembles de vecteurs réussis (§2) ;
- toutes les capacités optionnelles activées, telles que les clés COSE, les clés de chiffrement, les validateurs de profil (profile validators) ou la récursion GTS imbriquée (nested-GTS recursion) ;
- la commande exacte ou le harnais de test utilisé pour produire le résultat.

La revendication n'est significative que pour le palier et l'ensemble de capacités nommés. Par exemple, un lecteur (Reader) Baseline peut réussir `baseline-reader` sans revendiquer la vérification de signature COSE, le déchiffrement, la récursion GTS imbriquée (nested-GTS recursion) ou l'application de la politique de profil (profile policy enforcement).

## 2. Sous-ensembles de vecteurs

Le corpus figé comporte actuellement un fichier d'octets de niveau supérieur `vectors/<id>.gts` et un
`vectors/<id>.expected.json` repli attendu par cas. Des sous-corpora JSON additionnels couvrent COSE,
Encrypt0, l'extraction de clés OpenPGP, emojihash et randomart. Ces sous-ensembles nommés sont les unités
utilisées par les tier claims :

| sous-ensemble | vecteurs | objectif |
|---|---|---|
| `wire-core` | `01-minimal`, `02-zstd-frame`, `06-header-tampered` | Grammaire d'en-tête/de trame, codecs obligatoires, CBOR déterministe et comportement de hachage d'en-tête. |
| `total-reader` | `03-unknown-codec`, `04-damaged-frame`, `05-torn-append`, `17-pre-segment-hard-fail`, `19-profile-union-opacity`, `28-empty-file`, `28b-non-header-item`, `28c-unsupported-version`, `28d-unknown-frame-type`, `28e-forward-term-reference`, `28f-malformed-transform-shape`, `28g-damaged-compressed-payload`, `28h-malformed-security-metadata` | Dégradation gracieuse, diagnostics, nœuds opaques, entrée tronquée (torn), comportement malformé/limite, en-têtes non pris en charge, charges utiles compressées endommagées, métadonnées de sécurité malformées et opacité des trames d'extension. |
| `graph-fold` | `09-suppression`, `11-datatype-defaulting`, `12-conflicting-reifier`, `13-position-constraint`, `14-bnode-label`, `15-two-segment-union`, `15b-anon-bnode-union`, `16-composed-round-trip`, `18-cross-segment-suppression`, `22-inline-blob` | Repli de graphe central, égalité de valeur, annotations/réificateurs, suppressions, blobs et union multi-segments. |
| `profile-layout` | `20-language-tag-discipline`, `21-degenerate-composition`, `23-files-profile-tree`, `24-files-profile-dedup`, `25-streamable-source`, `25b-streamable-compacted`, `26-streamable-lie`, `27-streamable-tail` | Conventions de profil, comportement de profil archive/fichiers, disposition diffusable en continu, compaction et cas de refus des outils de publication. Le garde actif `scripts/interop.sh` ajoute des preuves de commande pack/unpack/diff `files` multi-moteurs pour ce sous-ensemble. |
| `okf-bundle` | `vectors/okf/*` répertoires de bundles Markdown | Fixtures d'importation de profil OKF, attentes de graphe replié et comportement de sidecar non mappé pour les outils conscients des profils. |
| `tar-archive` | `vectors/tar/*.tar`, `vectors/tar/*.tar.gz`, `vectors/tar/*.tar.zst` | Fixtures de transformation d'import/export Tar, incluant les projections d'archives positives et les cas de refus d'archives non sécurisées. |
| `resilience-negative` | `03-unknown-codec`, `04-damaged-frame`, `05-torn-append`, `06-header-tampered`, `17-pre-segment-hard-fail`, `19-profile-union-opacity`, `21-degenerate-composition`, `26-streamable-lie`, `28-empty-file`, `28b-non-header-item`, `28c-unsupported-version`, `28d-unknown-frame-type`, `28e-forward-term-reference`, `28f-malformed-transform-shape`, `28g-damaged-compressed-payload`, `28h-malformed-security-metadata` | Superposition d'audit pour les entrées de haut niveau contradictoires : CBOR tronqué, trames endommagées, compression endommagée, mauvaises limites de segment, métadonnées de transformation/profil/sécurité malformées, entrée vide/sans en-tête et comportement de refus/diagnostic de taille limitée. |
| `streaming-property` | chaque `vectors/*.gts` de haut niveau, testé à chaque limite d'élément CBOR | Totalité du repli de préfixe et croissance monotone du repli pour les lecteurs en continu. |
| `corpus-generator-determinism` | chaque `vectors/*.gts` de haut niveau | Reproductibilité du générateur de référence pour le corpus figé, incluant les fixtures intentionnellement endommagées, tronquées (torn), altérées et malformées. Cela prouve la répétabilité de la construction du corpus, et non la conformité du rédacteur public. |
| `writer-determinism` | sorties de rédacteur de haut niveau valides, incluant `25b-streamable-compacted` comme oracle d'octets de compaction diffusable en continu et `29-deterministic-writer` comme oracle d'octets de création de graphe | Sortie de rédacteur public reproductible, hachages déterministes, création de graphe déterministe et compaction déterministe sous paramètres fixes. Les fixtures de corpus négatives NE DOIVENT PAS (MUST NOT) utiliser ce sous-ensemble. |
| `crypto-cose` | `vectors/cose/*.json`, `vectors/signed/basic.json` | Sérialisation COSE Sign1, signatures par trame et comportement de vérification de signature. |
| `crypto-encrypt` | `vectors/encrypt0/basic.json` | Comportement de scellement/ouverture COSE Encrypt0 pour les moteurs qui implémentent le chiffrement. |
| `crypto-deferred` | `vectors/crypto-deferred/*.json` | Descripteurs de contrat multi-destinataires différés `COSE_Encrypt` et ECDH-ES+A256KW. Ces vecteurs empêchent les revendications de support prématurées ; ce ne sont pas des vecteurs d'implémentation v1 tant que des fixtures au niveau de l'octet et des harnais d'interopérabilité ne remplacent pas les espaces réservés. |
| `openpgp-transport-key` | `vectors/openpgp/*.json` | Extraction de clé de transport OpenPGP intégrée et accord de fingerprint/emojihash multi-moteurs. |
| `human-hash` | `vectors/emojihash/*.json`, `vectors/randomart/*.json` | Rendu de condensé (digest) destiné aux humains utilisé par les CLI et les outils de publication. |
| `security-policy` | `vectors/security/*.json` | Séparation de la politique de confiance du profil, destinataires opaques pseudonymes et cas négatifs de limite de récursion GTS imbriquée. |
| `advanced-index-proof` | `vectors/proofs/*.json` plus les fichiers indexés créés par l'implémentation | Pré-images MMR stables, vérification JSON de preuve d'inclusion détachée, rejet de mauvaise preuve et diagnostics de lecteur `index.mmr` optionnels. |

Un échelon PEUT (MAY) exiger un sous-ensemble plus des assertions spécifiques au mode supplémentaires. Par exemple, `profile-layout` contient des fichiers que les lecteurs permissifs peuvent replier (fold) en tant qu'octets GTS locaux, alors que les outils de validation doivent également refuser des violations spécifiques de publish-class ou verify-class.

Les manifestes délimités validés (committed) regroupent ces sous-ensembles par surface de conformité : `vectors/manifest.core.json` contient le corpus de lecteur/rédacteur de format filaire principal, `vectors/manifest.profiles.json` contient les fixtures de profil et de politique de profil, `vectors/manifest.transforms.json` contient les fixtures de transformation/outil, et `vectors/manifest.json` demeure le manifeste agrégé pour les vérifications à l'échelle du dépôt.

Le sous-ensemble `resilience-negative` est une superposition d'audit, et non un échelon distinct. Chaque entrée est un vecteur GTS de premier niveau, est marquée comme négative, est maintenue dans une taille d'octets validée limitée, et possède une attente de manifeste qui documente soit des diagnostics, soit un résultat de refus. Puisque les harnais de moteur complet du dépôt énumèrent le `vectors/*.gts` de premier niveau, Python, Rust, Go, TypeScript, Kotlin et Smalltalk consomment les mêmes fichiers d'octets négatifs de résilience et les comparent avec les mêmes résultats `*.expected.json`. Les fixtures de politique de sécurité JSON restent dans `security-policy` pour les assertions de politique de confiance sensible au profil et de récursivité GTS imbriquées.

## 3. Niveaux

| niveau | sous-ensembles et vérifications requis | chaîne de revendication |
|---|---|---|
| Lecteur de base (Baseline Reader) | `wire-core`, `total-reader`, `graph-fold`, et leur superposition `resilience-negative` de base en mode de lecture permissive (permissive-read) ; les correspondances JSON du graphe attendues ; les diagnostics correspondent ; les entrées malformées ne déclenchent jamais de panique ou d'interruption du processus. | `GTS Baseline Reader, corpus <commit>` |
| Lecteur en continu (Streaming Reader) | Lecteur de base plus `streaming-property` ; l'implémentation expose une API de destination (sink) sans matérialisation qui émet des événements de repli (fold) locaux au segment tout en préservant les diagnostics finaux et les têtes de segment. La mémoire retenue est censée être limitée par `O(distinct terms + maximum decoded frame size + validation sidecar state)`, et non par les triplets repliés ou les blobs. | `GTS Streaming Reader, corpus <commit>` |
| Lecteur complet (Full Reader) | Lecteur de base plus les sous-ensembles optionnels implémentés, au minimum `crypto-cose` pour la vérification de signature si le support de signature est revendiqué, `crypto-encrypt` si le support de déchiffrement est revendiqué, `security-policy` lors de la revendication de la récursion GTS imbriquée (nested-GTS), et le comportement d'index/MMR lorsqu'il est présent. | `GTS Full Reader (<capabilities>), corpus <commit>` |
| Rédacteur (Writer) | Les octets émis sont déterministes là où la spécification requiert une sortie déterministe, et les fichiers créés par le rédacteur répondent aux attentes du Lecteur de base. La génération reproductible de montages (fixtures) de corpus intentionnellement invalides est couverte par `corpus-generator-determinism` et n'implique pas une conformité publique de Rédacteur (Writer). | `GTS Writer, corpus <commit>` |
| Outil de validation (Validating Tool) | Lecteur de base plus les modes de vérification stricte (strict verify) et de vérification de classe de publication (publish-class verify) (§7) ; les vecteurs de refus `profile-layout` produisent les résultats non nuls/de refus requis. | `GTS Validating Tool, corpus <commit>` |
| Outil sensible aux profils (Profile-Aware Tool) | Outil de validation plus le validateur de profil nommé ; les diagnostics et avertissements spécifiques au profil correspondent au contrat du profil. | `GTS Profile-Aware Tool (<profile>), corpus <commit>` |
| Outil de transformation (Transform Tool) | L'opération de transformation ou d'archivage nommée effectue un cycle complet (round-trip) ou refuse les montages (fixtures) selon son contrat de profil/outil sans revendiquer le déterminisme du Rédacteur (Writer) de base. | `GTS Transform Tool (<transform>), corpus <commit>` |

Dans ce dépôt, Go, Rust et TypeScript revendiquent actuellement le niveau Lecteur en continu (Streaming Reader) pour des API de destination (sink) spécifiques. Go utilise `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)`. Rust utilise `gmeow_gts::reader::read_to_sink_from_reader(reader, ReadOptions, sink)`. TypeScript utilise l'exportation pour navigateur `foldStreamToSink(stream, options)`. Ces API lisent à partir d'entrées de flux/lecteur et émettent des événements de destination (sink) sans matérialiser l'union du graphe replié, les triplets repliés ou la table de données (payload) des blobs. Leurs barrières de corpus vérifient les codes de diagnostic finaux, les têtes de segment, les profils, les métadonnées, l'état de la disposition diffusable en continu (streamable-layout) et le nombre d'événements de repli locaux au segment par rapport à des oracles de lecteur complet ou de lecteur de segment.

Le module hérité `read_to_sink(&[u8], ...)` de Rust reste une enveloppe (wrapper) de compatibilité pour les appelants qui détiennent déjà les octets. `foldStream(stream, options)` et `readStream(stream, options)` de TypeScript restent des commodités de navigateur retournant des graphes. Python ne fournit actuellement que des preuves de repli de préfixe (prefix-fold) et de lecteur complet. Les revendications futures pour des API additionnelles DOIVENT (MUST) inclure un chemin de destination (sink) sans matérialisation ainsi que des preuves de mémoire correspondant à la limite ci-dessus.

Un outil peut revendiquer plusieurs niveaux. Un paquet en ligne de commande qui expose les commandes d'archive `read`, `verify`, `compact` et `files` pourrait revendiquer les niveaux Lecteur de base (Baseline Reader), Rédacteur (Writer), Outil de validation (Validating Tool), Outil sensible aux profils (Profile-Aware Tool) (`files`) et Outil de transformation (Transform Tool) (`tar`), tout en ne revendiquant pas le niveau Lecteur complet (Full Reader) s'il ne peut pas déchiffrer ou effectuer une récursion dans les blobs GTS imbriqués.
La matrice d'API et de commandes multilingue pour ces surfaces publiques est maintenue dans [`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md).
Les reports (deferrals) concernant les destinations de flux (streaming sink) avancées, les index/MMR/preuves, la réplication, la récupération par plage (range-fetch) et les tests de performance (benchmark) sont maintenus dans [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md).
Le contrat concernant la politique de confiance/profil, le budget de GTS imbriqué (nested-GTS) et le report de cryptographie est maintenu dans [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md).

## 4. Format de graphe attendu

Le corpus de haut niveau actuel utilise `vectors/<id>.expected.json`, généré par
`python/src/gts/vectors.py::expected_for`. Les implémentations DOIVENT (MUST) comparer les mêmes champs, à moins
que le manifeste ne restreigne explicitement un vecteur :

```json
{
  "mode": "default",
  "diagnostics": ["UnknownCodec"],
  "terms": 3,
  "quads": 1,
  "segments": 1,
  "segment_heads": ["0123..."],
  "profiles": ["generic"],
  "streamable": [
    {"claimed": false, "covered": 0, "tail": 0}
  ],
  "opaque_reasons": ["unknown-codec"],
  "suppressions": 0,
  "blobs": {
    "blake3:...": {"size": 13, "mt": "text/html"}
  },
  "nquads": [
    "<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> \"Cat\"@en ."
  ]
}
```

Sémantique des champs :

| champ | signification |
|---|---|
| `mode` | Mode de lecture utilisé par le vecteur. Les valeurs JSON attendues actuelles sont `default` (lecture permissive) et `pre-segment` ; les valeurs du manifeste utilisent les noms explicites du §7. |
| `diagnostics` | Liste ordonnée des codes de diagnostic émis par le lecteur (reader). Les détails de diagnostic ne sont pas figés dans le corpus actuel. |
| `terms`, `quads`, `segments`, `suppressions` | Sommaires des décomptes repliés (folded). |
| `segment_heads` | Identifiants d'en-tête de segment hexadécimaux dans l'ordre du fichier. La dernière valeur est l'en-tête de fichier pour la vérification à en-tête unique. |
| `profiles` | Déclarations de profil (profile) de segment repliées (folded) à partir des en-têtes. |
| `streamable` | État de disposition (layout) par segment : indicateur de revendication (claim flag), nombre de trames (frames) couvertes et nombre de queues accrétives. |
| `opaque_reasons` | Chaînes de raison de nœud opaque (opaque node) triées. |
| `blobs` | Condensé (digest) de blob en ligne associé au type de média déclaré et à la taille en octets décodée. |
| `nquads` | Lignes de projection RDF triées à partir du graphe replié (folded). Les étiquettes de nœuds vides (blank-node) sont censées correspondre au moteur de rendu de référence, à moins que le manifeste ne déclare une comparaison d'isomorphisme uniquement. |

## 5. Schéma de manifeste de vecteurs

Le dépôt intègre les manifestes portables pour le corpus figé :

- `vectors/manifest.core.json` : vecteurs de base de format filaire pour lecteur (reader)/rédacteur (writer) pour les revendications de Baseline Reader, de Streaming Reader et du rédacteur (Writer) de base.
- `vectors/manifest.profiles.json` : montages (fixtures) de disposition de profil (profile-layout), de paquets OKF et de politiques de sécurité (security-policy) pour les outils de validation et sensibles aux profils (profile-aware).
- `vectors/manifest.transforms.json` : montages pour les transformations et outils d'archive tar.
- `vectors/manifest.json` : manifeste agrégé utilisé par les vérifications à l'échelle du dépôt et les rapports de publication (release reports).

Ces manifestes rendent explicite l'ancienne convention de paires de fichiers pour les vecteurs d'octets de haut niveau et nomment les sous-corpus JSON utilisés par les vérifications facultatives de cryptographie, human-hash, OpenPGP, de signature (signed), de profil (profile) et de sécurité. Chaque manifeste utilise cette forme :

```json
{
  "schema": "https://blackcatinformatics.ca/gts/vector-manifest/v1",
  "manifest_version": 1,
  "manifest_scope": "core",
  "corpus_revision": "git:<commit>",
  "generated_by": "gts.vectors",
  "vectors": [
    {
      "id": "03-unknown-codec",
      "title": "unknown codec degrades to opaque node",
      "input": {
        "path": "vectors/03-unknown-codec.gts",
        "media_type": "application/vnd.blackcat.gts+cbor-seq"
      },
      "mode": "permissive-read",
      "negative": true,
      "required_capabilities": ["cbor", "blake3", "identity"],
      "subsets": ["total-reader"],
      "tiers": ["baseline-reader"],
      "expected": {
        "graph": "vectors/03-unknown-codec.expected.json",
        "diagnostics": ["UnknownCodec"],
        "expected_head": "<hex-or-null>",
        "opaque_reasons": ["unknown-codec"]
      },
      "notes": "Reader must keep chain/fold total and surface the undecodable frame."
    }
  ]
}
```

Les manifestes archivés utilisent
`"corpus_revision": "git:repository-commit-containing-manifest"` comme espace réservé (placeholder) délibéré.
Cet espace réservé évite les hachages de validation (commit hashes) auto-référencés dans les fichiers qui contiennent le hachage. Il est valide pour la validation du dépôt, mais il ne s'agit pas d'un identifiant de conformité de publication (release conformance identifier).

Les versions candidates (Release candidates) et les rapports de conformité tiers DOIVENT (MUST) remplacer l'espace réservé au moment du rapport par une révision `git:` exacte. La révision DOIT (MUST) être soit un identifiant de validation (commit id) complet de 40 caractères qui se résout dans le dépôt, soit une étiquette (tag) Git locale. Ne modifiez pas manuellement le manifeste archivé pour cela ; générez un artefact de manifeste de publication estampillé :

```bash
python scripts/check_vector_manifest.py \
  --release-manifest dist/vector-manifest.release.json
```

Cette commande valide le corpus et écrit une copie du manifeste dont `corpus_revision` nomme la validation (commit) `HEAD` actuelle. Pour estampiller une étiquette de publication (release tag) ou une validation (commit) explicite à la place, passez `--corpus-revision git:<tag-or-full-commit>`. La commande
`python scripts/check_vector_manifest.py` simple continue de valider les manifestes archivés avec espaces réservés. `python scripts/check_vector_manifest.py --write` réécrit tous les manifestes archivés à partir de l'arborescence de montages (fixture tree).

Les champs de manifeste de haut niveau requis sont `schema`, `manifest_version`, `manifest_scope`, `corpus_revision`, `generated_by` et `vectors`. `manifest_scope` est l'un des suivants : `aggregate`, `core`, `profiles` ou `transforms`.

Champs de vecteurs requis :

| champ | exigence |
|---|---|
| `id` | Identifiant de vecteur stable ; DEVRAIT (SHOULD) correspondre au nom de base du fichier. |
| `input.path` | Chemin vers les octets d'entrée canoniques ou la fixture JSON. |
| `mode` | L'un des `permissive-read`, `strict-verify`, `publish-verify`, `profile-verify`, `pre-segment`, ou une extension définie par le profil. |
| `negative` | `true` lorsque le vecteur attend des diagnostics, un refus, un statut de vérification non nul ou une violation de profil. |
| `required_capabilities` | Noms de capacités requis pour exercer le vecteur, tels que `zstd`, `cose-sign1`, `encrypt0`, `cose-encrypt`, `ecdh-es+a256kw`, `openpgp`, `streamable-index`, ou `files-profile`. |
| `subsets` | Un ou plusieurs noms de sous-ensembles du §2. |
| `tiers` | Noms de paliers du §3 qui consomment le vecteur. |
| `expected.graph` | Chemin JSON du graphe attendu, ou `null` pour les fixtures JSON qui ne sont pas des graphes. |
| `expected.diagnostics` | Liste des codes de diagnostic attendus dans l'ordre d'émission du lecteur. |
| `expected.expected_head` | Hexadécimal de la tête de segment ou du fichier final attendu lorsque le vecteur en affirme un ; `null` lorsqu'il n'est pas affirmé. |
| `notes` | Explication humaine du comportement épinglé. |

Les champs de vecteur optionnels incluent `expected.segment_heads`, `expected.exit_code`,
`expected.stderr_contains`, `expected.signature_status`, `expected.profile_findings`,
`compare.nquads` (`exact` ou `bnode-isomorphism`), et `links` vers les sections de la spécification.

## 6. Registre des diagnostics

Les codes de diagnostic constituent une API stable. Les mises en œuvre PEUVENT (MAY) ajouter des détails, des index de trames, des identifiants de segments ou des champs spécifiques au profil (profile), mais NE DOIVENT PAS (MUST NOT) renommer ces codes lorsqu'elles revendiquent le niveau (tier) qui les possède.

Valeurs de gravité :

- `fatal` : aucun graphe complet ne peut être replié (folded) pour le mode demandé ou aucun contenu ultérieur ne peut être interprété en toute sécurité.
- `error` : le lecteur (reader)/outil peut généralement retourner un repli (fold) partiel, mais la vérification stricte échoue.
- `warning` : la lecture permissive réussit et la vérification stricte PEUT (MAY) réussir si le mode déclare la condition comme non fatale.
- `info` : observation lisible par machine qui ne fait pas échouer la vérification par elle-même.

| code | gravité | s'applique à | comportement du lecteur | récupérable ? | raison opaque | niveau requis |
|---|---|---|---|---|---|---|
| `EmptyFile` | fatal | structure de fichier | Retourne un graphe/résultat vide et un diagnostic. | non | aucun | Baseline Reader |
| `DamagedFrame` | erreur | hachage d'en-tête/trame, décodage de charge utile, charge utile malformée | Isole l'élément endommagé lorsque possible, affiche un diagnostic et replie les survivants lorsque les limites sont connues. | partiel | `damaged` lorsqu'il est représenté comme opaque | Baseline Reader |
| `BrokenChain` | erreur | chaîne id/prev | Affiche la rupture de chaîne ; la vérification stricte échoue. | partiel | aucun | Baseline Reader |
| `TornAppendError` | avertissement | élément CBOR incomplet à la fin | Ignore les octets incomplets à la fin et replie le dernier préfixe complet. | oui | aucun | Baseline Reader |
| `UnknownCodec` | avertissement | capacité de transformation | Préserve la trame (frame) comme opaque et continue de replier le contenu connu. | oui | `unknown-codec` | Baseline Reader |
| `MissingKey` | avertissement | transformation chiffrée | Préserve la trame (frame) comme opaque et continue de replier le contenu connu. | oui | `missing-key` | Full Reader lorsque la prise en charge du déchiffrement est revendiquée |
| `KeyWrapFailed` | avertissement | transformation chiffrée multi-destinataire différée | Préserve la trame (frame) comme opaque lorsque les métadonnées de destinataire ECDH ou le déballage AES-KW échouent. | oui | `missing-key` | Future Full Reader lorsque la prise en charge `cose-encrypt`/ECDH est revendiquée |
| `ConflictingReifier` | erreur | repli du graphe | Garde la première liaison dans l'ordre du fichier et ignore la liaison conflictuelle. | oui | aucun | Baseline Reader |
| `PositionConstraint` | erreur | repli du graphe | Rejette la ligne incriminée et continue de replier les autres lignes/trames. | oui | aucun | Baseline Reader |
| `ForwardReference` | erreur | dictionnaire de termes | Abandonne ou ignore la référence avant non valide et continue de replier en toute sécurité. | oui | aucun | Baseline Reader |
| `SegmentBoundary` | fatal | mode de compatibilité pré-segment | Arrête avant de mal replier un segment ultérieur en tant qu'identifiants globaux au fichier. | non pour ce mode | aucun | Baseline Reader compatibility test |
| `IllTypedLiteral` | avertissement | import de syntaxe RDF/XSD | Préserve la forme lexicale littérale et le type de données verbatim ; expose un diagnostic et/ou un fichier d'accompagnement de métadonnées `gts:illTypedLiterals`. | oui | aucun | RDF codec / Profile-Aware Tool |
| `TruncatedLog` | erreur | tête attendue / fraîcheur | Replie les octets observés mais fait échouer la vérification par rapport à la tête demandée. | oui | aucun | Full Reader or Validating Tool |
| `StreamableLayoutError` | erreur | revendication de disposition diffusable | Replie les octets mais fait échouer la vérification stricte/de profil pour la revendication de disposition. | oui | aucun | Validating Tool |
| `IndexMmrError` | erreur | racine MMR d'index optionnel | Replie les octets mais fait échouer la vérification stricte pour l'engagement d'index. | oui | aucun | Full Reader lorsque la prise en charge MMR/preuve est revendiquée |
| `RecursionLimit` | erreur | récursion GTS imbriquée | Arrête la récursion et expose le contenu imbriqué comme indisponible/opaque. | oui | défini par la mise en œuvre | Full Reader |
| `UnknownFrameType` | avertissement | trame d'extension | Préserve la vérification de la chaîne ; ignore ou affiche opaque/diagnostic jusqu'à ce qu'un profil (profile) le gère. | oui | `unknown-frame-type` si opaque | Profile-Aware Tool |

Les validateurs de profil (profile) PEUVENT (MAY) définir des codes de diagnostic supplémentaires spécifiques au profil, mais ils DOIVENT (MUST) utiliser un espace de noms de profil ou documenter le code dans la spécification du profil.

## 7. Modes de lecture et de vérification

| mode | objectif | comportement | preuves de test |
|---|---|---|---|
| `permissive-read` | Lecture/repli (fold) de bibliothèque pour les consommateurs qui souhaitent le meilleur graphe récupérable. | Ne jamais paniquer sur des entrées de corpus malformées ; retourner l'état du graphe plus les diagnostics/nœuds opaques ; les diagnostics n'empêchent pas de retourner un résultat. | `wire-core`, `total-reader` et `graph-fold` en tant qu'attentes de graphe replié de base ; `profile-layout` comme preuve de profil/outil. |
| `strict-verify` | Vérificateur de transport pour les contrôles de chaîne/hachage/disposition/signature demandés par l'appelant. | Quitter/échouer sur toute erreur ou diagnostic fatal ; PEUT (MAY) autoriser des avertissements documentés tels que des profils non pris en charge si le mode les déclare comme avertissements. | Tests CLI `verify`, `04`, `05`, `06`, `17`, `26`, tests signed/head. |
| `publish-verify` | Barrière de publication et de réécriture pour les commandes qui créent ou distribuent des artefacts. | Refuser les artefacts structurellement valides mais invalides selon la politique, tels que la composition à repli vide, la composition tout-supprimer, les mensonges diffusables en continu, l'extraction non sécurisée ou la compaction non reproductible. | `21-degenerate-composition`, `22-inline-blob`, `25b-streamable-compacted`, `26-streamable-lie`. |
| `profile-verify` | Validation sensible au profil au-dessus de la validité du format filaire de base. | Appliquer le vocabulaire du profil, la capacité, la confiance, la disposition et les règles d'archive sans redéfinir la validité GTS de base. | `19-profile-union-opacity`, `20-language-tag-discipline`, `23-files-profile-tree`, `24-files-profile-dedup`, `25`-`27`. |

Les noms de mode sont des valeurs de manifeste, pas nécessairement des sous-commandes CLI littérales. Une CLI PEUT (MAY) exposer
plusieurs modes via une seule commande avec des drapeaux ; le harnais de test DOIT (MUST) enregistrer quel mode a été utilisé.

## 8. Rapports

Un rapport de conformité DEVRAIT (SHOULD) inclure :

- le nom de l'implémentation, la version, le commit, le système d'exploitation et l'architecture ;
- la révision exacte du corpus ou l'étiquette utilisée pour le rapport, correspondant au manifeste de version estampillé ;
- les tier claims et les sous-ensembles de vecteurs ;
- les lignes de commande ou les noms de tests ;
- le décompte des réussites/échecs par sous-ensemble ;
- tout vecteur ignoré avec le nom de la capacité manquante ;
- les diagnostics émis pour les vecteurs ayant échoué ;
- si le corpus a été régénéré et s'est avéré reproductible.

Les rapports DEVRAIENT (SHOULD) être des artefacts de construction durables pour les versions candidates et DEVRAIENT (SHOULD) être joints aux notes de version pour la v1.0 et les versions ultérieures.
