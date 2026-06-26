<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/gts-reference.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Implémentation de référence GTS Python (`gts`)

> Traduction informative de [`docs/gts-reference.md`](../../../../docs/gts-reference.md). Le document anglais demeure la source normative pour les règles de compatibilité, les déclarations de conformité, les matrices de parité, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

Un lecteur/rédacteur léger en dépendances pour le format filaire **Graph Transport Substrate** spécifié dans [`GTS-SPEC.md`](./GTS-SPEC.md). Le paquet `gts` (PyPI : `gmeow-gts`) est le niveau **baseline** : il valide la spécification de manière empirique et constitue la source unique de vérité pour le corpus de conformité neutre vis-à-vis des langages par rapport auquel les moteurs Rust, Go et TypeScript effectuent également leurs validations. Les déclarations de niveau et les sous-ensembles de vecteurs sont définis dans [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md).

## Ce qui est couvert

- **Journal CBOR en ajout seulement** + l'état replié RDF 1.2 (`terms` / `quads` /
  `reifies` / `annot`, blobs, métadonnées, suppressions, nœuds opaques et diagnostics),
  et le repli `snapshot` (§7).
- **Intégrité** — CBOR déterministe + auto-`id` BLAKE3 par trame et la chaîne de
  content-id `prev`, avec la règle de pré-image d'en-tête-genèse (§5, §9.1).
- **Catalogue de transformations** — `identity` / `gzip` / `zstd` ; le modèle de capacité dégrade
  un codec inconnu ou un codec `encrypt` (aucune clé dans la base de référence) en un **nœud opaque**
  plutôt que d'échouer la lecture (§8, §7.6).
- **Robustesse** — détection d'ajout incomplet (torn-append) (§3), isolation de trames endommagées et les
  diagnostics canoniques (§2.4), incluant `EmptyFile`, `TornAppendError`,
  `DamagedFrame`, `BrokenChain`, `UnknownCodec`, `MissingKey`, `ConflictingReifier`,
  `PositionConstraint`, `ForwardReference`, `SegmentBoundary` et `UnknownFrameType`.
- **Interopérabilité `RDF -> GTS`** — avec l'extension optionnelle `[rdf]` (rdflib), un
  `Graph`/`Dataset` rdflib (graphe de base RDF 1.1) peut être incorporé dans un dictionnaire GTS
  avec `gts.from_rdflib` ; `gts.to_rdflib` est strict quant aux limitations des
  triplets cités (quoted-triple) de RDF 1.2. Le contrat d'intégration se trouve dans
  [GTS-ECOSYSTEM-INTEGRATIONS.md](./GTS-ECOSYSTEM-INTEGRATIONS.md).
- **Transformations sortantes** — `gts → nquads` (§14) et `gts → {sqlite,duckdb}` (le
  chargement relationnel encodé par dictionnaire avec identifiants entiers ; le moteur résout les identifiants via une jointure).
- **Signature COSE (§9.2)** — `Writer(signer=…)` signe chaque trame en COSE_Sign1 sur son
  `id` (EdDSA/Ed25519) ; `read(data, keys=…)` vérifie et enregistre le statut par trame
  dans `Graph.signatures` (`valid`/`invalid`/`unverified` sous un `KeyProvider`). Plus
  la **détection de troncature** via `read(data, expected_head=…)` → `TruncatedLog` (#272).
- **Chiffrement COSE (§9.3)** — `Writer.add_frame(…, encrypt=(kid, key))` scelle une
  charge utile en tant que `COSE_Encrypt0` (la transformation la plus externe) et enregistre le destinataire ;
  `read(data, keys=…)` décrypte lorsque la clé de contenu est détenue, sinon la trame se replie en
  un **nœud opaque** `missing-key` avec son destinataire visible (l'invariant d'opacité) —
  divulgation sélective (#272).

## Pas encore (suivis sous #267)

Multi-destinataire / ECDH key-wrap (ceci livre le destinataire unique `COSE_Encrypt0`) ;
le contrat trust/profile-policy v1 (signatures-required, pseudonymous-`kid`,
et récursion GTS imbriquée bornée) est suivi dans
[`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md). Les reports restants incluent
l'accélération `index`/MMR (§6.2), un chargement de base de données par diffusion de trames (frame-streaming) pour les entrées très volumineuses, et l'expansion du vocabulaire de packaging.

## Utilisation

```python
from gts import Writer, Term, TermKind, read, to_nquads

w = Writer(profile="dist")
w.add_terms([
    Term(TermKind.IRI, "https://example.org/Cat"),
    Term(TermKind.IRI, "http://www.w3.org/2000/01/rdf-schema#label"),
    Term(TermKind.LITERAL, "Cat", lang="en"),
])
w.add_quads([(0, 1, 2, None)])
data = w.to_bytes()                      # the GTS file (bytes)

graph = read(data)                       # parse + verify chain + fold
print(to_nquads(graph))                  # <…/Cat> <…/label> "Cat"@en .
```

CLI (`pip install gmeow-gts` installe le binaire `gts`) :

```bash
gts info   file.gts             # frame/term/quad/blob counts + diagnostics
gts fold   file.gts             # fold to N-Quads on stdout
gts verify file.gts             # verify chains; exit 1 on any diagnostic
gts cat -o combined.gts a.gts b.gts   # validating composer
gts pack ./my-dir -o archive.gts      # package a directory (files profile)
gts unpack archive.gts -C ./restore   # extract a files profile
```

La forme de l'API multi-moteur, la matrice de parité CLI et les écarts de commandes uniquement Python sont maintenus dans
[`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md).
La diffusion en continu avancée, les index/MMR/proof, la réplication, le range-fetch et les reports de benchmark sont suivis
dans [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md).

## Conformité

`python/tests/test_gts.py` implémente le sous-ensemble non-COSE du corpus de conformité défini dans [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md) (fichier minimal, trames `zstd`/`gzip`, unknown-codec → opaque, trame endommagée, ajout tronqué, hachage d'en-tête, suppression, attribution de type de données par défaut, réificateur conflictuel, contraintes de position, localité des nœuds vides, blob en ligne, repli de snapshot). Un lecteur conforme du profil de base est intentionnellement petit — c'est tout l'intérêt du format.
