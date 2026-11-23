<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-SECURITY-POLICY.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Politique de sécurité et de confiance GTS

> Traduction informative de [`docs/GTS-SECURITY-POLICY.md`](../../../../docs/GTS-SECURITY-POLICY.md). Le document anglais demeure la source faisant autorité pour la gouvernance, la sécurité, les versions, les licences, la contribution, les obligations de conduite, les processus de divulgation et les commandes exécutables. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

Ce document fixe le contrat de sécurité v1 qui se situe au-dessus du format de transmission (wire format) GTS de base. Le lecteur (reader) de base vérifie les octets, les hachages de trame (frame hashes), les chaînes et la validité cryptographique COSE facultative. Il ne décide pas si un signataire est autorisé, si une affirmation est vraie ou si un identifiant de destinataire préserve la vie privée.

## Séparation de la confiance

`Signature.status == "valid"` signifie que la signature COSE est vérifiée à l'aide d'une clé résolue par l'appelant. Cela ne signifie pas :

- la clé est approuvée par le déploiement ;
- le signataire est autorisé pour le profil ;
- l'allégation RDF signée est vraie.

La confiance du déploiement est représentée par `gts.policy.TrustPolicy` en Python,
`gmeow_gts::policy::TrustPolicy` en Rust,
`go.blackcatinformatics.ca/gts/policy.TrustPolicy` en Go et
`policy.TrustPolicy` en TypeScript. La vérification de fichier de haut niveau est exposée sous la forme
`gts.verify.verify_file` en Python et `gmeow_gts::verify::verify_file` en Rust ;
tous les moteurs qui exposent une politique de profil signalent l'état de la signature cryptographique séparément de la confiance du déploiement. Un outil sensible aux profils peut exiger un signataire de confiance, tandis qu'un lecteur de base peut toujours renvoyer le graphe récupérable ainsi que l'état de la signature.

Les déploiements Rust qui nécessitent une politique basée sur un fichier activent `--features policy-config`.
Cette fonctionnalité optionnelle ajoute des assistants de chargement JSON et `gts verify --policy <file>` ;
`--features policy-config-yaml` ajoute YAML par-dessus. Les versions Rust par défaut conservent l'évaluateur de politique, mais n'héritent pas des dépendances serde ou de l'analyseur YAML. La forme du fichier est :

```yaml
trusted_signers:
  - did:example:issuer
require_trusted_signer: true
pseudonymous_kid_pattern: "^anon:[0-9a-fA-F]{32,}$"
```

## Application des profils

Les paliers de conformité de la V1 sont intentionnellement distincts :

- Lecteur de base (Baseline Reader) : analyse et replie les données GTS récupérables, les blobs, les signatures, les diagnostics et les métadonnées de segment. Il n'effectue pas de récursion dans les blobs GTS imbriqués et n'autorise pas les signataires.
- Lecteur complet (Full Reader) : comprend le comportement du Lecteur de base plus des capacités optionnelles telles que la vérification de signature, les contrôles de déchiffrabilité et la découverte limitée de GTS imbriqués.
- Outil sensible aux profils (Profile-Aware Tool) : comprend la sortie du lecteur plus des contrôles de politique de déploiement/profil tels que les signataires de confiance, les engagements de tête de preuve (evidence head), la pseudonymie des destinataires opaques, les déclarations de vocabulaire de profil et les revendications de disposition diffusable en continu.

| Profil | Application dans la v1 | Codes de constat |
|---|---|---|
| `evidence` | Nécessite des trames signées et une tête de segment signée lors de la vérification du profil. La confiance de déploiement est facultative à moins que l'appelant ne fournisse des identifiants de signataires de confiance. | `ProfileSignatureRequired`, `ProfileSignatureInvalid`, `ProfileSignatureUnverified`, `EvidenceHeadCommitmentRequired`, `ProfileSignerUntrusted` |
| `opaque` | Nécessite des trames signées lors de la vérification du profil. Les valeurs de destinataire à haute confidentialité `kid` DOIVENT (MUST) être pseudonymes : motif par défaut `anon:[0-9a-fA-F]{32,}`. | `ProfileSignatureRequired`, `OpaqueRecipientKidMissing`, `OpaqueRecipientKidPublic` |
| `bundle` | Les blobs GTS imbriqués sont un comportement optionnel du Lecteur complet. Les lecteurs de base les traitent comme des blobs ordinaires. Les lecteurs complets DOIVENT (MUST) appliquer des budgets de récursion et de taille décodée. | `RecursionLimit` |
| `files` / `stream` | Les contrôles existants de vocabulaire de profil et de disposition diffusable en continu restent une politique de profil/outil, et non une validité fondamentale. | `ProfileVocabularyUndeclared`, `ProfileVocabularyUnused`, `StreamVocabularyWithoutLayout`, `StreamableLayoutError` |

## Budgets GTS imbriqués

Les appelants du lecteur complet (Full Reader) utilisent `gts.read_nested(...)` en Python,
`gmeow_gts::nested::read_nested(...)` en Rust,
`nested.ReadNested(...)` en Go, ou `nested.readNested(...)` en TypeScript pour
effectuer une récursion dans les blobs dont le type de média déclaré est
`application/vnd.blackcat.gts+cbor-seq`. Le résultat expose les sous-graphes imbriqués par le
condensé du blob contenant. La récursion s'arrête lorsque `max_depth` / `maxDepth` ou
`max_decoded_bytes` / `maxDecodedBytes` est dépassé et enregistre
`RecursionLimit`.

## Différés cryptographiques

| Capacité | Décision du palier v1 |
|---|---|
| COSE_Sign1 / Ed25519 | Mise en œuvre de la capacité optionnelle Full Reader et de l'entrée de politique de profil. |
| COSE_Encrypt0 / AES-256-GCM | Mise en œuvre de la capacité optionnelle Full Reader pour un destinataire direct. |
| Enveloppes multi-destinataires COSE_Encrypt | Différé en dehors de la conformité v1. Aucun moteur NE PEUT (MAY) le revendiquer tant que les vecteurs au niveau de l'octet et les tests d'interopérabilité ne sont pas arrivés. Le contrat du descripteur réside dans `vectors/crypto-deferred/*.json`. |
| Emballage de clé ECDH / ECDH-ES+A256KW | Différé en dehors de la conformité v1. Le support futur utilise `COSE_Encrypt` avec le chiffrement de contenu `A256GCM`, la gestion des clés de destinataire `ECDH-ES+A256KW` et l'emballage de clé de contenu `A256KW`. |
| Politique d'identifiant de destinataire pseudonyme | Mise en œuvre en tant que politique de profil pour le profil `opaque`. |

Les modes de défaillance `cose-encrypt` différés sont corrigés avant que tout moteur PEUT (MAY) revendiquer le support :

- deux destinataires ou plus PEUVENT (MAY) déballer la même clé de chiffrement de contenu ;
- aucune correspondance de clé de destinataire détenue n'enregistre `MissingKey` et préserve l'opacité de `reason:"missing-key"` ;
- une mauvaise clé, un en-tête de destinataire ECDH malformé ou un échec de déballage/authentification AES-KW enregistre `KeyWrapFailed` et préserve l'opacité de `reason:"missing-key"` ;
- aucun mode de défaillance NE PEUT (MAY) exposer le texte en clair ou convertir l'autorisation de déploiement en validité cryptographique.

## Vecteurs

Les descripteurs de vecteurs de sécurité enregistrés se trouvent dans `vectors/security/` :

- `nested-recursion-limit.json` enregistre le comportement négatif `RecursionLimit`
  requis pour la récursion GTS imbriquée. `nested-recursion-limit.gts.hex` est la
  fixture d'octets promue utilisée par les tests du lecteur imbriqué TypeScript.
- `profile-policy.json` enregistre les résultats de confiance/profil prouvant que la
  validité cryptographique, la confiance de déploiement et la vérité de l'allégation sont distinctes.
- `nested-duplicate-digest.gts.hex` enregistre la fixture de budget de condensé imbriqué en double
  utilisée pour prouver que le contenu imbriqué partagé est comptabilisé une seule fois.

Les tests unitaires Python, Rust, Go et TypeScript instancient ces vecteurs directement
là où chaque moteur expose l'API pertinente. Les vecteurs d'octets inter-moteurs peuvent être
promus dans le corpus de niveau supérieur une fois que davantage de moteurs consomment le même
format de fixture à partir du manifeste.
