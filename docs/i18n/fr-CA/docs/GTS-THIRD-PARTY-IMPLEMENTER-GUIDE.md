<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Guide de l'implémenteur tiers GTS

> Traduction informative de [`docs/GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md`](../../../../docs/GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md). Le document anglais demeure la source normative pour les règles de compatibilité, les déclarations de conformité, les matrices de parité, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

Ce guide s'adresse aux implémenteurs qui souhaitent créer un lecteur (reader) GTS indépendant et faire une déclaration de conformité Baseline Reader testable. Il n'est pas normatif. Le format filaire demeure défini par [`GTS-SPEC.md`](./GTS-SPEC.md), et les déclarations de conformité demeurent définies par [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md).

Utilisez ce guide comme ordre de construction et liste de contrôle :

1. Implémentez le plus petit lecteur (reader) capable d'analyser, de vérifier et de replier (fold) le corpus de conformité (conformance corpus) de référence.
2. Portez le harnais vector-manifest.
3. Publiez une déclaration de conformité qui nomme la révision exacte du corpus et la commande.

## Ancrages normatifs

Ne copiez pas ces règles dans un guide d'implémentation ou un fichier README en tant qu'exigences indépendantes. Créez un lien vers la section propriétaire et implémentez en fonction de celle-ci.

| Sujet | Propriétaire normatif |
|---|---|
| Structure de fichiers, segments et composition par ajout (cat-append) | [`GTS-SPEC.md` sections 3 et 3.1](./GTS-SPEC.md#3-file-structure) |
| Conventions CBOR et encodage déterministe | [`GTS-SPEC.md` section 4](./GTS-SPEC.md#4-cbor-conventions) |
| Mappe d'en-tête (Header map) | [`GTS-SPEC.md` section 5](./GTS-SPEC.md#5-header) |
| Mappe de trames (Frame map) et résolution de charge utile (payload) | [`GTS-SPEC.md` sections 6 et 6.1](./GTS-SPEC.md#6-frames) |
| Modèle de graphe et algorithme de repli (fold) | [`GTS-SPEC.md` section 7](./GTS-SPEC.md#7-graph-data-model-and-fold) |
| Nœuds opaques (Opaque nodes) | [`GTS-SPEC.md` section 7.6](./GTS-SPEC.md#76-opaque-nodes) |
| Codecs obligatoires | [`GTS-SPEC.md` section 8.4](./GTS-SPEC.md#84-mandatory-core-set-and-durability) |
| Vérification de la chaîne de trames | [`GTS-SPEC.md` section 9.1](./GTS-SPEC.md#91-per-frame-self-hash-and-content-id-chain-mandatory) |
| CDDL complet | [`GTS-SPEC.md` section 21](./GTS-SPEC.md#21-complete-cddl-appendix) |
| Hachage, signature et pré-images de clés d'extension | [`GTS-SPEC.md` section 22](./GTS-SPEC.md#22-hash-signature-and-extension-key-preimages) |
| Niveau du lecteur (Reader) de base (Baseline) | [`GTS-CONFORMANCE.md` section 3](./GTS-CONFORMANCE.md#3-tiers) |
| Schéma de manifeste de vecteurs | [`GTS-CONFORMANCE.md` section 5](./GTS-CONFORMANCE.md#5-vector-manifest-schema) |
| Registre de diagnostics | [`GTS-CONFORMANCE.md` section 6](./GTS-CONFORMANCE.md#6-diagnostics-registry) |
| Modes de lecture et de vérification | [`GTS-CONFORMANCE.md` section 7](./GTS-CONFORMANCE.md#7-read-and-verify-modes) |
| Enregistrement de profil (profile) tiers | [`GTS-SPEC.md` section 13](./GTS-SPEC.md#13-profiles) |

## Travail minimal du lecteur de base (Baseline Reader)

Un lecteur de base (Baseline Reader) est la plus petite mise en œuvre indépendante utile. Il lit les octets GTS en mode de lecture permissive (permissive-read), vérifie la chaîne d'identifiants de contenu (content-id) suffisamment loin pour faire remonter des diagnostics, replie le contenu de graphe récupérable et préserve le contenu inconnu ou non pris en charge en tant que nœuds opaques (nœuds opaques).

Travail minimal :

- Analyser une séquence CBOR (CBOR Sequence), et non un objet CBOR unique pour l'ensemble du fichier.
- Détecter les en-têtes de segment (segment headers) et les trames (frames).
- Accepter l'étiquette d'autodescription CBOR optionnelle `55799` lorsqu'elle étiquette un en-tête de segment.
- Décoder les formes d'en-tête (Header) et de trame (Frame) de l'annexe CDDL.
- Recalculer les identifiants d'en-tête (Header) et de trame (Frame) en utilisant la table de préimage (preimage table).
- Vérifier le lien `prev` de chaque trame par rapport à l'identifiant de l'élément précédent dans le même segment.
- Mettre en œuvre la pile de transformation obligatoire : `identity`, `gzip` et `zstd`.
- Replier `terms`, `quads`, les réificateurs (reifiers), les annotations, les suppressions, les objets binaires (blobs), les métadonnées, les diagnostics, les registres de segment (segment ledgers), les signatures et les nœuds opaques selon l'algorithme de repli (fold algorithm).
- Retourner des diagnostics plutôt que de déclencher une panique (panic) lors d'entrées de corpus de conformité malformées.
- Préserver le contenu indécodable, non pris en charge, chiffré sans clé ou endommagé en tant que nœuds opaques lorsque la récupération est possible.
- Comparer votre sortie repliée aux champs JSON attendus nommés par le manifeste de vecteurs (vector manifest).

Un lecteur de base (Baseline Reader) n'a pas besoin de mettre en œuvre :

- La vérification de signature COSE ou le support du chiffrement.
- L'extraction de clés OpenPGP.
- La récursion GTS imbriquée (Nested-GTS).
- La validation de preuves MMR/index.
- Les événements de flux (Stream events).
- Le déterminisme du rédacteur (Writer).
- L'outillage de publication strict (Strict publish tooling).
- La validation de politique sensible au profil (Profile-aware policy validation).
- Les aides pour base de données, Parquet, magasin d'objets (object-store) ou récupération par plage (range-fetch).

Ces capacités peuvent être ajoutées ultérieurement et revendiquées sous le niveau (tier) approprié : lecteur en continu (Streaming Reader), lecteur complet (Full Reader), rédacteur (Writer), outil de validation (Validating Tool) ou outil sensible au profil (Profile-Aware Tool).

## Pipeline de lecteur suggéré

L'API exacte est spécifique à l'implémentation, mais ce pipeline correspond aux documents de conformité :

```text
bytes
  -> CBOR Sequence item iterator
  -> segment boundary detector
  -> Header validator
  -> Frame validator
  -> transform resolver
  -> frame-payload decoder
  -> fold accumulator
  -> Graph plus diagnostics, segment heads, opaque nodes, and metadata
```

Pseudocode :

```text
read_gts(bytes):
  items = parse_cbor_sequence(bytes)
  result = empty_graph()
  current_segment = none
  previous_id = none

  for item in items:
    if is_segment_header(item):
      current_segment = validate_header(item)
      previous_id = current_segment.id
      result.segments.append(current_segment.summary)
      continue

    frame = validate_frame_envelope(item, previous_id)
    previous_id = frame.id

    if frame.envelope_is_damaged:
      result.add_diagnostic("DamagedFrame")
      result.add_opaque(frame, reason="damaged")
      continue

    payload = resolve_transforms(frame)
    if payload.is_unsupported:
      result.add_diagnostic(payload.diagnostic)
      result.add_opaque(frame, reason=payload.opaque_reason)
      continue

    fold_payload(result, frame.type, payload)

  return result
```

Les propriétés importantes sont la totalité et l'observabilité : les entrées du corpus malformées ou non prises en charge DOIVENT (MUST) retourner un résultat avec des diagnostics plutôt que d'interrompre le processus.

## Utilisation de `vectors/manifest.core.json`

Le manifeste principal est l'index de conformité portable du lecteur (lecteur) de base (Baseline). Il nomme le fichier d'entrée, le JSON du graphe attendu, les capacités requises, les sous-ensembles, les paliers (tiers), les diagnostics et les notes pour chaque vecteur. L'agrégat `vectors/manifest.json` inclut également des fixtures optionnelles de profil (profil), de transformation, de cryptographie (crypto), de preuve et de hachage humain (human-hash) qui sont utiles pour les vérifications complètes du dépôt, mais ne constituent pas le point de départ du lecteur (lecteur) de base (Baseline).

Commencez par les vecteurs du manifeste principal dont le `tiers` contient `baseline-reader` :

```bash
python - <<'PY'
import json
from pathlib import Path

manifest = json.loads(Path("vectors/manifest.core.json").read_text())
for vector in manifest["vectors"]:
    if "baseline-reader" in vector["tiers"]:
        expected = vector["expected"].get("graph")
        print(vector["id"], vector["input"]["path"], expected)
PY
```

Pour chaque vecteur sélectionné :

1. Lisez `input.path` en tant qu'octets.
2. Exécutez le lecteur (lecteur) en mode lecture permissive (permissive-read).
3. Chargez `expected.graph` lorsqu'il n'est pas `null`.
4. Comparez les champs attendus nommés par le manifeste : décomptes, diagnostics, têtes de segment (segment heads), raisons opaques (opaque reasons), résumés de blob (blob summaries), état diffusable en continu (streamable state), profils (profils) et N-Quads.
5. Traitez `negative: true` comme « comportement attendu de diagnostic/refus », et non comme « le processus devrait échouer ou paniquer ».
6. Enregistrez les vecteurs sautés uniquement lorsque `required_capabilities` nomme une capacité en dehors du palier (tier) revendiqué.

Validez le manifeste lui-même avant de l'utiliser comme artefact de version ou de rapport :

```bash
python scripts/check_vector_manifest.py
python scripts/check_vector_manifest.py --self-test
```

Les rapports de version NE DEVRAIENT PAS (SHOULD NOT) citer le paramètre fictif (placeholder) archivé `git:repository-commit-containing-manifest` comme corpus. Estampillez une révision exacte pour les rapports :

```bash
python scripts/check_vector_manifest.py \
  --release-manifest dist/vector-manifest.release.json
```

## Comparaison JSON attendue

Le corpus de haut niveau actuel compare les résumés de graphes repliés plutôt qu'un modèle d'objet interne privé. Une implémentation peut utiliser ses propres structures de données tant qu'elle peut émettre des champs équivalents.

Comparez au moins :

- `diagnostics` : liste ordonnée de codes de diagnostic.
- `terms`, `quads`, `segments` et `suppressions` : résumés de décomptes repliés.
- `segment_heads` : identifiants de tête de segment dans l'ordre du fichier.
- `profiles` : déclarations de profil repliées.
- `streamable` : état de la disposition par segment.
- `opaque_reasons` : motifs d'opacité triés.
- `blobs` : résumés de condensé de blob en ligne, de type de média et de taille décodée.
- `nquads` : lignes de projection RDF triées.

Les étiquettes de nœuds vierges sont censées correspondre au moteur de rendu de référence, à moins que le manifeste ne restreigne un vecteur à une comparaison d'isomorphisme uniquement. Consultez le format de graphe attendu dans la [`GTS-CONFORMANCE.md` section 4](./GTS-CONFORMANCE.md#4-expected-graph-format).

## Diagnostics et nœuds opaques

Les diagnostics font partie du comportement public d'une revendication de conformité. Ne renommez pas les codes appartenant au niveau (tier) que vous revendiquez.

Les diagnostics du Baseline Reader incluent les comportements liés à des entrées malformées ou hostiles tels que `EmptyFile`, `DamagedFrame`, `BrokenChain`, `TornAppendError`, `UnknownCodec`, `ConflictingReifier`, `PositionConstraint`, `ForwardReference` et `SegmentBoundary`.

Le comportement des nœuds opaques est ce qui permet au lecteur de rester total :

- Codec inconnu : préservez la trame en tant que nœud opaque avec `reason:"unknown-codec"`.
- Clé de déchiffrement manquante : préservez la trame en tant que nœud opaque avec `reason:"missing-key"` lorsque le support de chiffrement est présent mais que la clé ne l'est pas.
- Trame récupérable endommagée : isolez le contenu endommagé comme opaque lorsque les limites des éléments sont connues.
- Type de trame structurelle inconnu : préservez la vérification de la chaîne et ignorez la charge utile ou présentez-la comme opaque jusqu'à ce qu'un profil pris en charge la traite. `UnknownFrameType` est un diagnostic de Profile-Aware Tool dans le registre de conformité, et non une partie de la chaîne de revendication du Baseline Reader.

Un nœud opaque n'est pas une perte de données. C'est une déclaration lisible par machine indiquant que le lecteur a transporté un contenu qu'il n'a pas pu décoder ou interpréter de manière sécurisée.

## Principes de base de l'enregistrement de profil

Les profils se situent au-dessus du format filaire de base. Un profil de domaine peut définir le vocabulaire, les règles de validation, la politique de confiance, le flux de travail de publication et les vecteurs spécifiques au profil, mais il NE DOIT PAS (MUST NOT) modifier :

- La grammaire de l'en-tête ou de la trame.
- La détection des limites de segment.
- Content-id, signature ou préimages de hachage.
- Résolution du catalogue de transformations.
- Sémantique de repli déterministe.

Un lecteur de base (Baseline Reader) DEVRAIT (SHOULD) exposer les déclarations et les exigences de profil sous forme de métadonnées repliées, de diagnostics ou de raisons opaques. Il n'est pas nécessaire d'appliquer la politique de profil pour revendiquer la conformité en tant que lecteur de base (Baseline Reader).

Les profils tiers DEVRAIENT (SHOULD) publier les champs d'enregistrement de profil énumérés dans la [`GTS-SPEC.md` section 13](./GTS-SPEC.md#13-profiles), y compris un jeton stable ou un URI, le propriétaire/contrôleur de changement, l'objectif, les vocabulaires requis, les règles de validation, la taxonomie des défaillances, les considérations de sécurité et de confidentialité, la politique de versionnage et les vecteurs de conformité.

## Exemple de revendication de lecteur de base (Baseline Reader)

```text
Implementation: ExampleGTS Reader 0.1.0
Conformance tier: GTS Baseline Reader
Corpus revision: git:0123456789abcdef0123456789abcdef01234567
Read mode: permissive-read
Vector subsets passed: wire-core, total-reader, graph-fold
Capabilities enabled: cbor, blake3, identity, gzip, zstd
Command: example-gts-conformance --manifest vectors/manifest.core.json --tier baseline-reader
Skipped vectors: none for the claimed tier
Optional capabilities not claimed: signatures, encryption, nested GTS, MMR proofs, profile policy
```

Si vous ne revendiquez qu'un sous-ensemble du comportement du lecteur de base (Baseline Reader), ne l'appelez pas conformité au lecteur de base (Baseline Reader). Utilisez une expression propre à l'implémentation telle que « lecteur expérimental » jusqu'à ce que les sous-ensembles requis soient validés.

## Pièges courants

- Traiter le fichier comme un seul objet CBOR englobant au lieu d'une séquence CBOR (CBOR Sequence).
- Hacher le Header avec la clé `id` incluse.
- Hacher une trame (frame) avec les clés `id` ou `sig` incluses.
- Abandonner les codecs inconnus au lieu de préserver les nœuds opaques (nœuds opaques).
- Faire échouer le processus sur les vecteurs négatifs au lieu de retourner des diagnostics.
- Ignorer les clés d'extension inconnues lors du recalcul des préimages.
- Traiter les échecs de politique de profil (profil) comme une invalidité du format binaire de base (wire-format).
- Revendiquer un comportement de lecteur (lecteur) complet parce que les signatures sont analysées, même lorsque la vérification de signature, la résolution de clé ou le comportement de politique de confiance sont manquants.
- Comparer uniquement les N-Quads tout en ignorant les diagnostics, les raisons opaques (opaque reasons), les têtes de segment (segment heads), l'état diffusable en continu (streamable state), les profils (profils) et les résumés de blob (blob summaries).
