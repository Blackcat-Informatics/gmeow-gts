<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-V1-RC1-CHECKLIST.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Liste de contrôle et ensemble d'artefacts GTS v1.0-rc1

> Traduction informative de [`docs/GTS-V1-RC1-CHECKLIST.md`](../../../../docs/GTS-V1-RC1-CHECKLIST.md). Le document anglais demeure la source faisant autorité pour la gouvernance, la sécurité, les versions, les licences, la contribution, les obligations de conduite, les processus de divulgation et les commandes exécutables. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md) et reste informative.

Cette liste de contrôle transforme le chemin de version (release) v1.0-rc1 dans [`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md) en un enregistrement de candidat à la version (release-candidate) exécutable. Copiez les sections non cochées dans le signalement de version (release issue) ou conservez une copie remplie comme un artefact de version (release artifact). Ne modifiez pas le manifeste de vecteurs (vector manifest) validé pour marquer une révision de version (release revision) ; générez l'artefact marqué décrit ci-dessous.

## 1. Dossier du candidat

| Champ | Valeur |
|---|---|
| Ticket de version | |
| PR de version | |
| Nom du candidat | `v1.0-rc1` |
| Version du paquet de version | |
| Commit de la spécification | |
| Révision du corpus | |
| Artéfact du manifeste de vecteurs | `dist/v1.0-rc1/vector-manifest.release.json` |
| Paquet/étiquette Rust | `gmeow-gts` / `rust-v<version>` |
| Paquet/étiquette Python | `gmeow-gts` / `py-v<version>` |
| Module/étiquette Go | `go.blackcatinformatics.ca/gts` / `go/v<version>` |
| Paquet/étiquette npm | `@blackcatinformatics/gmeow-gts` / `npm-v<version>` |
| Paquet/étiquette Ruby | `gmeow-gts` / `ruby-v<version>` |
| Commit des notes de version | |
| Responsable de la version | |
| Date | |

Avant de choisir une version de package de pré-publication, vérifiez que la chaîne est acceptée par Cargo, PyPI, npm et le repository version guard. Si la syntaxe de l'écosystème diffère pour la même version candidate (release candidate), consignez ici les versions exactes par écosystème et mettez à jour le release guard avant le tagging. Une discordance tag/manifeste est un bloqueur de version (release blocker) car les workflows de tag vérifient les versions du manifeste avant la publication.

## 2. Classification des points bloquants

Classez chaque constat par rapport aux listes d'éléments bloquants et non bloquants dans
[`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md#71-v10-rc1-blockers).

### 2.1 Bloquants v1.0-rc1

| Classe de bloqueur | Preuves requises | État |
|---|---|---|
| Aucun changement intentionnel au format de transmission (wire-format) ne subsiste | Problèmes GIP/spécification ouverts examinés ; aucun changement accepté ne modifie la grammaire d'en-tête/trame, les préimages de hachage, les préimages de signature, la composition de segment, la résolution de transformation ou la sémantique de repli (fold) de base. | |
| Les vecteurs de référence sont présents et figés | Le manifeste de vecteurs est validé ; la régénération du corpus ne présente aucun diff ; le manifeste de version (release manifest) estampillé indique la révision du corpus. | |
| Le comportement de référence multi-moteur réussit | Les vérifications Rust, Python, Go, TypeScript, Smalltalk/Pharo, Kotlin/JVM et d'interopérabilité réussissent par rapport à la même révision du corpus. | |
| La couverture des tests de fumée de l'enveloppe C ABI réussit | `rust/capi` plus les tests de fumée des enveloppes C++, .NET, PHP, Lua, Swift, Ruby, R et Julia réussissent par rapport au candidat de version (release candidate). | |
| La politique de registre est publiée | Les types de trames, diagnostics, codecs, profils, cibles de transformation et espaces de noms réservés sont couverts dans les documents de gouvernance/spécification/conformité. | |
| Le modèle de sécurité est clair | La politique de sécurité et les gardes de report (deferral) crypto réussissent ; aucune vulnérabilité élevée ou critique de l'analyseur, de la cryptographie, de l'extraction ou du pipeline de version (release-pipeline) ne reste ouverte. | |
| Le type de média et les conseils de distribution sont présents | `application/vnd.blackcat.gts+cbor-seq`, le comportement HTTP/range, la publication immuable et les conseils de vérification d'artefact sont présents dans la spécification/documentation. | |
| Le langage de compatibilité est clair | Les règles de compatibilité de transmission (wire), de corpus, de paquet et de profil sont présentes dans la gouvernance et citées par les notes de version (release notes). | |
| La revue des responsables d'implémentation n'a aucun résultat bloquant | Les problèmes/commentaires de revue sont fermés, différés en tant que non-bloquants, ou enregistrés ci-dessous avec le propriétaire et la justification. | |
| Le remboursement du budget de qualité est enregistré | La PR de version (release PR) réduit au moins un point chaud au-delà de la cible vers `target_lines`, ou enregistre une exception délibérée avec le propriétaire, la justification et le problème de suivi ; aucune augmentation de la base de référence n'est acceptée sans l'étiquette de revue du budget de qualité ou une note de revue d'architecture. | |

### 2.2 Éléments non bloquants adjacents à la version

Ces éléments sont utiles pour l'adoption, mais ne retardent pas la rc1 lorsque la conformité de base est prête, à moins qu'ils ne révèlent l'un des éléments bloquants ci-dessus.

| Livrable | Problème de suivi | État | Note de version |
|---|---|---|---|
| Guide d'implémentation tiers | `#104` | | |
| Suite de bancs d'essai et rapport de version | `#105` | `just bench-release` et [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md) | |
| Ébauche de l'article GTS | `#106` | [`GTS-PAPER-DRAFT.md`](./GTS-PAPER-DRAFT.md) | Récit informatif de l'article ; ne constitue pas un texte de spécification normatif. |
| Achèvement du profil standard optionnel | | | |
| Base de données, Parquet, navigateur, magasin d'objets, range-fetch, MMR, réplication ou outils de preuve avancés | | | |
| Gestion future des clés ou enveloppes de chiffrement multi-destinataires | | | |
| Alias de paquets neutres ou soumission à un organisme de normalisation | | | |

## 3. Instantané de l'environnement local

Consigner l'état de la chaîne d'outils et du dépôt utilisé pour le candidat.

```bash
git status --short --branch
git rev-parse HEAD
git tag --points-at HEAD
rustc --version
cargo --version
python --version
uv --version
go version
node --version
npm --version
```

## 4. Configuration de l'ensemble d'artéfacts

Créez un répertoire local pour l'ensemble avant d'exécuter les vérifications des candidats.

```bash
export RC=v1.0-rc1
export OUT="dist/${RC}"
rm -rf "${OUT}"
mkdir -p "${OUT}/reports" "${OUT}/packages" "${OUT}/sbom" "${OUT}/attestations"
git rev-parse HEAD > "${OUT}/spec-commit.txt"
git rev-parse HEAD > "${OUT}/corpus-revision.txt"
python scripts/check_vector_manifest.py \
  --release-manifest "${OUT}/vector-manifest.release.json"
```

Si la révision du corpus est une étiquette ou un commit explicite plutôt que le
`HEAD` actuel, marquez-la explicitement :

```bash
python scripts/check_vector_manifest.py \
  --corpus-revision git:<tag-or-full-commit> \
  --release-manifest "${OUT}/vector-manifest.release.json"
```

L'ensemble devrait contenir :

- `spec-commit.txt` avec le commit complet contenant la spécification de version (release spec).
- `corpus-revision.txt` avec le commit ou l'étiquette (tag) du corpus.
- `vector-manifest.release.json` avec le `corpus_revision` estampillé.
- Journaux de test et de conformité par moteur dans `reports/`.
- Résultats des essais à blanc (dry-run) des paquets dans `packages/`.
- SBOM des flux de travail et références d'attestation après la publication de l'étiquette (tag).
- Notes de version ou un lien vers la RP de version (release PR).

## 5. Contrôles de garde et de dérive

Exécutez-les depuis la racine du dépôt.

```bash
bash scripts/check-versions.sh
python scripts/check_cli_parity.py
python scripts/check_api_parity.py
python scripts/check_advanced_contract.py
python scripts/check_ecosystem_contract.py
python scripts/check_security_contract.py
python scripts/check_crypto_deferrals.py
python scripts/check_quality_budget.py
python scripts/check_vector_manifest.py
python scripts/check_vector_manifest.py --self-test
```

Confirmez que les ancres de politique rc1 sont présentes :

```bash
rg -n "v1.0-rc1|v1.0-rc1 Blockers|v1.0-rc1 Non-Blockers" docs/GTS-GOVERNANCE.md
rg -n "Wire-format compatibility|Corpus compatibility|Package compatibility|Profile compatibility" docs/GTS-GOVERNANCE.md
rg -n "application/vnd.blackcat.gts\\+cbor-seq|HTTP range|immutable" README.md docs/GTS-SPEC.md
rg -n "corpus_revision|release manifest|conformance report" docs/GTS-CONFORMANCE.md vectors/manifest.json
```

## 6. Vérifications du corpus et de conformité

Régénérer et comparer le corpus validé :

```bash
just check-vectors
git diff --exit-code vectors
```

Exécuter les suites complètes du moteur et capturer les journaux :

```bash
cargo test --manifest-path rust/Cargo.toml --locked 2>&1 \
  | tee "${OUT}/reports/rust-test.log"
(
  cd python
  uv sync --extra rdf
  uv run pytest --junitxml "../${OUT}/reports/python-pytest.xml"
)
(
  cd go
  CGO_ENABLED=0 go test -json ./... \
    | tee "../${OUT}/reports/go-test.jsonl"
)
(
  cd ts
  npm ci
  npm test 2>&1 | tee "../${OUT}/reports/ts-test.log"
)
bash scripts/interop.sh 2>&1 | tee "${OUT}/reports/interop.log"
```

Exécutez les chemins de l'outil validating-tool et de refus de publication via les tests CLI dans les suites de moteurs ci-dessus. Les notes de version DEVRAIENT (SHOULD) indiquer quels niveaux de conformité sont revendiqués et citer la révision exacte du corpus.

Utilisez cette ligne de rapport pour chaque mise en œuvre :

| Implementation | Version | OS/arch | Tier claim | Corpus revision | Command/log | Pass/fail/skips |
|---|---|---|---|---|---|---|
| Rust | | | | | | |
| Python | | | | | | |
| Go | | | | | | |
| TypeScript | | | | | | |

## 7. Contrôles de sécurité et de la chaîne d'approvisionnement

Exécutez les contrôles locaux de la chaîne d'approvisionnement et d'hygiène du dépôt lorsque les outils sont disponibles. `just audit` exécute l'analyse de dépendances OSV définie dans le justfile ; pre-commit est une barrière distincte d'hygiène, de lint et d'analyse de secrets. La posture SLSA de la version est enregistrée dans [`GTS-RELEASE-SLSA.md`](./GTS-RELEASE-SLSA.md) : les attestations d'artéfacts GitHub actuelles sont traitées comme des preuves SLSA v1.0 Build Level 2. Ne revendiquez pas le niveau SLSA v1.0 Build Level 3 à moins que les voies de version ne soient passées à des workflows réutilisables renforcés et que les artéfacts représentatifs ne soient vérifiés par rapport à l'identité du workflow du signataire attendue.

```bash
just audit
pipx run pre-commit run --all-files
```

Inspectez les workflows de sécurité GitHub pour le commit candidat :

```bash
gh run list --workflow security.yml --branch main --limit 5
gh run list --workflow codeql.yml --branch main --limit 5
gh run list --workflow fuzz.yml --branch main --limit 5
```

Enregistrez toute vulnérabilité, tout résultat de CodeQL, de fuzz, de pipeline de version ou de signature comme un élément bloquant, à moins qu'il ne soit explicitement hors de portée pour la conformité de base v1.
Si une version adopte intentionnellement des workflows réutilisables pour l'alignement Build Level 3, enregistrez ici la politique de workflow du signataire et les preuves de vérification avant le marquage :

| Voie de version | Workflow réutilisable | Commande de vérification du signataire | État |
|---|---|---|---|
| Rust `gmeow-gts` | | | |
| Rust `visual-hashing` | | | |
| Python | | | |
| Go | | | |
| TypeScript | | | |

## 8. Essais à blanc des paquets

Ne créez pas de tag tant que tous les essais à blanc n'ont pas réussi à partir d'une branche de version (release) propre ou d'un commit de fusion de PR de version.

```bash
cargo package --manifest-path rust/Cargo.toml --locked
(
  cd python
  uv lock --check
  uv build --out-dir "../${OUT}/packages/python"
)
(
  cd ts
  npm ci
  npm run build
  npm pack --pack-destination "../${OUT}/packages/npm"
)
(
  cd go
  CGO_ENABLED=0 go build -trimpath -ldflags "-s -w" \
    -o "../${OUT}/packages/go/gts" ./cmd/gts
)
archive="$(bash rust/capi/scripts/package.sh)"
bash rust/capi/scripts/verify-archive.sh "${archive}"
GTS_PACKAGE_DRY_RUN_OUT="${OUT}/packages/wrappers" \
  bash scripts/package_dry_run_wrappers.sh
```

La simulation (dry-run) des wrappers couvre la liste des paquets Rust C ABI, la vérification de l'archive C ABI installable, la consommation de l'archive C++ installée, les tests de fumée (smoke tests) des consommateurs des gestionnaires de paquets Conan et vcpkg, l'empaquetage NuGet local .NET, la validation Composer, la génération de la racine du paquet PHP Packagist ainsi que les tests de fumée des consommateurs de dépôts de chemins locaux, le lint/make/pack de LuaRocks ainsi que l'exécution de fumée du rock installé, la validation dump/run du paquet racine Swift, la construction/installation de gemmes Ruby, la construction/vérification R et les tests de paquets Julia.

Pour la parité de version (release) Go, simulez (dry-run) également la forme de construction croisée (cross-build) utilisée par `.github/workflows/release-go.yaml` :

```bash
VERSION="<version>"
mkdir -p "${OUT}/packages/go-cross"
(
  cd go
  for os in linux darwin windows; do
    for arch in amd64 arm64; do
      ext=""
      [ "${os}" = windows ] && ext=".exe"
      CGO_ENABLED=0 GOOS="${os}" GOARCH="${arch}" \
        go build -trimpath -ldflags "-s -w" -o "gts${ext}" ./cmd/gts
      base="gts_${VERSION}_${os}_${arch}"
      if [ "${os}" = windows ]; then
        zip -qj "../${OUT}/packages/go-cross/${base}.zip" "gts${ext}"
      else
        tar czf "../${OUT}/packages/go-cross/${base}.tar.gz" "gts${ext}"
      fi
      rm -f "gts${ext}"
    done
  done
)
sha256sum "${OUT}"/packages/go-cross/* > "${OUT}/packages/go-cross/checksums.txt"
```

## 9. Notes de version

La PR de version DOIT (MUST) mettre à jour `CHANGELOG.md`, `CITATION.cff`, les manifestes de paquet, les fichiers de verrouillage (lockfiles) et les extraits README/docs lorsque les versions des paquets changent.

Les notes de version DOIVENT (MUST) inclure :

- le nom du candidat et la version du paquet ou les versions par écosystème ;
- le commit de la spécification (spec commit) ;
- la révision du corpus et le nom de l'artéfact du manifeste estampillé ;
- les revendications de niveau de conformité par mise en œuvre ;
- les noms des registres de paquets et les étiquettes de version (release tags) ;
- le résumé de l'examen des points bloquants (blocker review) ;
- le résumé de la réduction du budget qualité, ou une exception documentée avec le responsable et le lien vers le problème de suivi (follow-up issue) ;
- les points non bloquants adjacents à la version et les liens vers les problèmes de suivi ;
- les instructions de vérification de la SBOM et de l'attestation ;
- les limitations connues et les capacités différées (deferred capabilities).

Preuve minimale des notes de version :

```bash
bash scripts/check-versions.sh
rg -n "<version>|spec commit|corpus revision|conformance|SBOM|attestation" CHANGELOG.md README.md docs
```

## 10. Séquence d'étiquetage et de publication

Après la fusion de la PR de version (release), étiquetez le commit de fusion exact. Poussez les étiquettes de version (release tags) une par une pour que chaque flux de travail (workflow) déclenché par une étiquette reçoive son propre événement.

Avant de pousser les étiquettes Rust, confirmez que l'entrée de l'éditeur de confiance (Trusted Publisher) de crates.io pour `gmeow-gts` est active avec le propriétaire/dépôt `Blackcat-Informatics/gmeow-gts`, le flux de travail (workflow) `release-cargo.yaml` et l'environnement `(none)`. Le chemin de version (release) Rust normal utilise GitHub Actions OIDC et ne nécessite pas `CARGO_REGISTRY_TOKEN`.
Avant la première publication de la crate source `gmeow-gts-capi`, confirmez que le secret d'amorçage (bootstrap) `CARGO_REGISTRY_TOKEN` est disponible pour `release-cargo-capi.yaml`. Ce jeton est uniquement destiné à l'amorçage (bootstrap) initial de la crate. Une fois que la première version apparaît sur crates.io, soumettez et complétez la migration de publication de confiance (Trusted Publishing) subséquente pour `gmeow-gts-capi`, propriétaire/dépôt `Blackcat-Informatics/gmeow-gts`, flux de travail (workflow) `release-cargo-capi.yaml` et environnement `(none)`, à moins qu'un environnement de version (release) protégé ne soit ajouté.

Si la crate Rust `gmeow-gts` dépend d'une version `visual-hashing` plus récente, publiez d'abord cette crate à partir de son dépôt autonome. Son entrée de publication de confiance (Trusted Publisher) sur crates.io DOIT (MUST) utiliser le propriétaire/dépôt `Blackcat-Informatics/visual-hashing`, le flux de travail (workflow) `release.yml` et l'environnement `(none)`.
Avant de pousser les étiquettes (tags) RubyGems, confirmez que le Trusted Publisher RubyGems en attente pour `gmeow-gts` utilise owner/repo `Blackcat-Informatics/gmeow-gts`, workflow `release-rubygems.yaml` et environnement `(none)` à moins que la version (release) n'ajoute explicitement un environnement protégé.

Avant de pousser les étiquettes (tags) Go, confirmez que les versions (releases) immuables au niveau du dépôt sont activées :

```bash
gh api repos/Blackcat-Informatics/gmeow-gts/immutable-releases
```

```bash
MERGE_COMMIT="<full-merge-commit>"
VERSION="<version>"
git tag "rust-v${VERSION}" "${MERGE_COMMIT}"
git tag "py-v${VERSION}" "${MERGE_COMMIT}"
git tag "go/v${VERSION}" "${MERGE_COMMIT}"
git tag "npm-v${VERSION}" "${MERGE_COMMIT}"
git tag "capi-v${VERSION}" "${MERGE_COMMIT}"
git tag "ruby-v${VERSION}" "${MERGE_COMMIT}"
git tag "${VERSION}" "${MERGE_COMMIT}" # Swift Package Manager / Swift Package Index
git push origin "rust-v${VERSION}"
git push origin "py-v${VERSION}"
git push origin "go/v${VERSION}"
git push origin "npm-v${VERSION}"
git push origin "capi-v${VERSION}"
git push origin "ruby-v${VERSION}"
git push origin "${VERSION}"
```

Si `visual-hashing` a changé, publiez-le avant les tags `rust-v*` qui dépendent de la
nouvelle version du crate :

```bash
VISUAL_HASHING_VERSION="<visual-hashing-version>"
gh repo clone Blackcat-Informatics/visual-hashing ../visual-hashing
cd ../visual-hashing
git tag "v${VISUAL_HASHING_VERSION}" "<visual-hashing-merge-commit>"
git push origin "v${VISUAL_HASHING_VERSION}"
cd -
```

Surveiller les workflows de version :

```bash
gh run list --event push --limit 30
gh run list --workflow release-cargo.yaml --branch "rust-v${VERSION}" --limit 5
gh run list --workflow release-cargo-capi.yaml --branch "capi-v${VERSION}" --limit 5
gh run list --workflow release-pypi.yml --branch "py-v${VERSION}" --limit 5
gh run list --workflow release-go.yaml --branch "go/v${VERSION}" --limit 5
gh run list --workflow release-npm.yaml --branch "npm-v${VERSION}" --limit 5
gh run list --workflow release-capi.yaml --branch "capi-v${VERSION}" --limit 5
```

Une fois que la balise de version sémantique Swift simple existe, validez le paquet racine
et soumettez l'URL du dépôt avec le protocole et l'extension `.git` au Swift
Package Index :

```bash
diff -u rust/capi/include/gts.h swift/Sources/CGts/include/gts.h
swift package dump-package --package-path .
bash swift/scripts/smoke.sh
```

Soumettre :

```text
https://github.com/Blackcat-Informatics/gmeow-gts.git
```

Si `visual-hashing` a été publié, surveillez son flux de travail par étiquette :

```bash
gh run list --repo Blackcat-Informatics/visual-hashing --workflow release.yml --branch "v${VISUAL_HASHING_VERSION}" --limit 5
```

Si une étiquette a été poussée vers le mauvais commit ou avec la mauvaise version, arrêtez-vous et consignez une note d'incident de version avant de la supprimer ou de la recréer.

## 11. Vérification des artefacts publiés

Vérifiez les registres publics et les artefacts de version une fois les workflows terminés.
Le vérificateur de test de fumée (smoke test) du mainteneur effectue le téléchargement, le hachage, la provenance du registre,
GitHub SLSA, SPDX SBOM et les vérifications immutable-release à partir des surfaces publiques uniquement :

```bash
VISUAL_HASHING_VERSION="<visual-hashing-version>"
just verify-release-dry-run "${VERSION}" "${VISUAL_HASHING_VERSION}"
just verify-release "${VERSION}" "${VISUAL_HASHING_VERSION}"
```

Une fois les paquets du wrapper C ABI publiés, exécutez le vérificateur prenant en charge les wrappers :

```bash
just verify-wrapper-release-dry-run "${VERSION}" "${VISUAL_HASHING_VERSION}"
just verify-wrapper-release "${VERSION}" "${VISUAL_HASHING_VERSION}"
```

Le même vérificateur peut être exécuté depuis l'interface utilisateur de GitHub Actions avec le workflow manuel `Verify published release`. Activez `dry_run` avant que les identifiants ou la propagation du registre ne soient prêts, et activez `include_wrapper_packages` pour la passe du wrapper. Il téléverse `dist/release-verification/${VERSION}/release-verification-summary.md` et le rapport JSON correspondant. Le rapport maintient la sévérité pass/warn/fail séparée des valeurs de statut de version (release) telles que `published`, `pending`, `metadata-mismatch` et `missing`, de sorte que le délai de propagation ne soit pas confondu avec des métadonnées erronées ou des artefacts absents. `release-verification-summary.json` en tant qu'artefacts de workflow. Ne passez pas `--allow-legacy-release-gaps` pour les nouvelles versions (releases); cette dérogation est uniquement destinée à l'audit des versions antérieures au SBOM et au durcissement des versions immuables (immutable-release).

### 11. Artéfacts de distribution (C ABI / Natif)

| ID | Composant | Critère | État | Notes |
|:---|:---|:---|:---:|:---|
| 11.1 | C ABI | Les en-têtes utilisent strictement des types d'entiers à largeur fixe ([108]) et sont exempts de types [109] spécifiques à la plateforme. | [110] | [111] |
| 11.2 | Symboles | Les bibliothèques dynamiques n'exportent que le préfixe avec espace de noms [112] ; aucune pollution de l'espace de noms global. | [113] | [114] |
| 11.3 | Pkg-config | Les fichiers [115] sont générés et valides pour toutes les plateformes cibles. | [116] | [117] |

```bash
cargo search gmeow-gts --limit 1
cargo search gmeow-gts-capi --limit 1
python -m pip index versions gmeow-gts
npm view @blackcatinformatics/gmeow-gts version
dotnet nuget search Gmeow.Gts --source https://api.nuget.org/v3/index.json
composer show blackcatinformatics/gmeow-gts --available
curl -fsSL https://luarocks.org/manifest.json | python -m json.tool >/dev/null
gem info gmeow-gts --remote
curl -fsSL https://blackcat-informatics.r-universe.dev/src/contrib/PACKAGES
curl -fsSL https://raw.githubusercontent.com/JuliaRegistries/General/master/G/GmeowGTS/Package.toml
gh release view "go/v${VERSION}" \
  --json tagName,name,url,isDraft,isImmutable,isPrerelease,publishedAt
gh release view "capi-v${VERSION}" \
  --json tagName,name,url,isDraft,isImmutable,isPrerelease,publishedAt
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify "capi-v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
```

Les commandes de vérification destinées aux consommateurs, qui sont toutes couvertes par le vérificateur smoke du mainteneur, sont :

```bash
pypi-attestations verify pypi \
  --repository https://github.com/Blackcat-Informatics/gmeow-gts \
  "https://files.pythonhosted.org/.../gmeow_gts-${VERSION}-py3-none-any.whl"
npm audit signatures
gh attestation verify <downloaded-artifact> --repo Blackcat-Informatics/gmeow-gts
gh attestation verify <downloaded-artifact> \
  --repo Blackcat-Informatics/gmeow-gts \
  --predicate-type https://spdx.dev/Document/v2.3
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify "capi-v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify-asset "go/v${VERSION}" <downloaded-go-asset> \
  --repo Blackcat-Informatics/gmeow-gts
gh release verify-asset "capi-v${VERSION}" <downloaded-capi-asset> \
  --repo Blackcat-Informatics/gmeow-gts
```

### Durabilité des preuves de version

- [ ] Toutes les attestations de version (SLSA, %161%, %162%) sont archivées dans le tag de version.
- [ ] Les instructions de construction reproductible sont vérifiées pour le tag actuel.
- [ ] Les registres de provenance incluent le commit source exact et le workflow de construction.
| Surface | Artefact durable | Preuve d'attestation |
|---|---|---|
| Go | Archives GitHub Release immuables, `checksums.txt` et `sbom-go-gts.spdx.json` | Attestation GitHub release pour la version immuable ; attestations de provenance SLSA pour les actifs de la version ; attestations SBOM SPDX pour les archives de la version |
| C ABI | Archives GitHub Release immuables, `checksums.txt` et `sbom-gmeow-gts-capi.spdx.json` | Attestation GitHub release pour la version immuable ; attestations de provenance SLSA pour les actifs de la version ; attestations SBOM SPDX pour les archives de la version |
| crates.io `gmeow-gts` | Paquet `.crate` hébergé sur le registre | Attestations de provenance SLSA et SBOM SPDX dans le magasin d'attestations de GitHub |
| crates.io `gmeow-gts-capi` | Paquet `.crate` hébergé sur le registre | Attestations de provenance SLSA et SBOM SPDX dans le magasin d'attestations de GitHub ; jeton d'amorçage jusqu'à ce que le suivi de Trusted Publishing soit en place |
| PyPI | Wheel/sdist hébergé sur le registre | Attestations de publication PyPI plus attestations de provenance SLSA et SBOM SPDX de GitHub |
| npm | Tarball hébergé sur le registre | Provenance npm plus attestations de provenance SLSA et SBOM SPDX de GitHub |
| RubyGems | Paquet `.gem` hébergé sur le registre | Attestations de provenance SLSA et SBOM SPDX dans le magasin d'attestations de GitHub |
| NuGet `Gmeow.Gts` | Paquet `.nupkg` hébergé sur le registre | Métadonnées du registre et vérification du téléchargement du paquet ; wrapper source uniquement nécessitant l'hôte `libgts` |
| Packagist `blackcatinformatics/gmeow-gts` | Métadonnées de balise VCS de Packagist | Métadonnées du registre et vérification de la référence source ; wrapper source uniquement nécessitant l'hôte `libgts` |
| LuaRocks `gmeow-gts` | Rockspec/source rock hébergé sur le registre | Manifeste racine LuaRocks et vérification du téléchargement du rockspec ; wrapper source uniquement nécessitant l'hôte `libgts` |
| Swift Package Index | Balise de version sémantique du dépôt et URL de paquet SPI | Vérification de la balise Git et enregistrement d'URL SPI canonique ; wrapper source uniquement nécessitant l'hôte `libgts` |
| r-universe `gmeowgts` | Paquet source hébergé sur le registre | Index PACKAGES et vérification du téléchargement du tarball source ; paquet source nécessitant l'hôte `libgts` |
| Julia General `GmeowGTS` | Métadonnées de paquet du registre général | Vérification de l'identité et de la version du registre ; wrapper source uniquement nécessitant l'hôte `libgts` |
| Conan/vcpkg `gmeow-gts` | Tests à blanc locaux de première partie jusqu'à l'intégration en amont | Aucune preuve de registre public jusqu'à ce que les recettes en amont soient acceptées |
Télécharger des artéfacts représentatifs pour vérification :

```bash
mkdir -p \
  "${OUT}/packages/go-release" \
  "${OUT}/packages/npm" \
  "${OUT}/packages/python" \
  "${OUT}/packages/ruby" \
  "${OUT}/packages/rust" \
  "${OUT}/packages/wrappers"
gh release download "go/v${VERSION}" --dir "${OUT}/packages/go-release"

python -m pip download --no-deps --dest "${OUT}/packages/python" "gmeow-gts==${VERSION}"
npm pack "@blackcatinformatics/gmeow-gts@${VERSION}" \
  --pack-destination "${OUT}/packages/npm"
gem fetch gmeow-gts --version "${VERSION}" --clear-sources --source https://rubygems.org
mv "gmeow-gts-${VERSION}.gem" "${OUT}/packages/ruby/"
curl -L "https://crates.io/api/v1/crates/gmeow-gts/${VERSION}/download" \
  -o "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate"
curl -L "https://crates.io/api/v1/crates/gmeow-gts-capi/${VERSION}/download" \
  -o "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate"
curl -L "https://api.nuget.org/v3-flatcontainer/gmeow.gts/${VERSION}/gmeow.gts.${VERSION}.nupkg" \
  -o "${OUT}/packages/wrappers/Gmeow.Gts.${VERSION}.nupkg"
curl -L "https://luarocks.org/gmeow-gts-${VERSION}-1.rockspec" \
  -o "${OUT}/packages/wrappers/gmeow-gts-${VERSION}-1.rockspec"
curl -L "https://blackcat-informatics.r-universe.dev/src/contrib/gmeowgts_${VERSION}.tar.gz" \
  -o "${OUT}/packages/wrappers/gmeowgts_${VERSION}.tar.gz"
```

Vérifier la provenance SLSA par défaut sur les artefacts représentatifs et les manifestes de version Go :

```bash
for artifact in "${OUT}"/packages/go-release/gts_"${VERSION}"_*; do
  gh attestation verify "$artifact" --repo Blackcat-Informatics/gmeow-gts
done
for artifact in \
  "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate" \
  "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate" \
  "${OUT}"/packages/npm/*.tgz \
  "${OUT}"/packages/python/* \
  "${OUT}"/packages/ruby/*.gem; do
  gh attestation verify "$artifact" --repo Blackcat-Informatics/gmeow-gts
done
gh attestation verify "${OUT}/packages/go-release/checksums.txt" \
  --repo Blackcat-Informatics/gmeow-gts
gh attestation verify "${OUT}/packages/go-release/sbom-go-gts.spdx.json" \
  --repo Blackcat-Informatics/gmeow-gts
```

Ces commandes vérifient la posture actuelle d'attestation d'artefact Build Level 2.
Si le candidat adopte des flux de travail réutilisables pour une posture plus robuste, répétez les vérifications d'artefacts représentatifs avec la politique de signataire attendue, par exemple :

```bash
gh attestation verify <downloaded-artifact> \
  --repo Blackcat-Informatics/gmeow-gts \
  --signer-workflow <owner>/<repo>/.github/workflows/<workflow>.yml@<ref>
```

Vérifiez l'attestation de version Go immuable et chaque actif de version téléchargé :

```bash
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
for artifact in "${OUT}"/packages/go-release/*; do
  gh release verify-asset "go/v${VERSION}" "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts
done
```

Vérifier les attestations SPDX SBOM pour un artefact représentatif de chaque voie de version. Le générateur SBOM actuel émet du SPDX 2.3, ainsi le type de prédicat est `https://spdx.dev/Document/v2.3`; if the emitted `spdxVersion` change, mettre à jour la version du prédicat pour qu'elle corresponde.

```bash
SBOM_PREDICATE="https://spdx.dev/Document/v2.3"
for artifact in "${OUT}"/packages/go-release/gts_"${VERSION}"_*; do
  gh attestation verify "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts \
    --predicate-type "${SBOM_PREDICATE}"
done
for artifact in \
  "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate" \
  "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate" \
  "${OUT}"/packages/npm/*.tgz \
  "${OUT}"/packages/python/*; do
  gh attestation verify "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts \
    --predicate-type "${SBOM_PREDICATE}"
done
```

Enregistrer l'état final du registre :

| Surface | Version/étiquette attendue | Preuve | État |
|---|---|---|---|
| crates.io `gmeow-gts` | | | |
| crates.io `gmeow-gts-capi` | | | |
| PyPI `gmeow-gts` | | | |
| Version Go `go.blackcatinformatics.ca/gts` | | | |
| npm `@blackcatinformatics/gmeow-gts` | | | |
| NuGet `Gmeow.Gts` | | | |
| Packagist `blackcatinformatics/gmeow-gts` | | | |
| LuaRocks `gmeow-gts` | | | |
| Swift Package Index `Blackcat-Informatics/gmeow-gts` | | | |
| RubyGems `gmeow-gts` | | | |
| r-universe `gmeowgts` | | | |
| Julia General `GmeowGTS` | | | |
| État Conan/vcpkg `gmeow-gts` | | | |
| Attestations SBOM | | | |
| Attestations de provenance de construction | | | |

## 12. Décision finale

La version candidate n'est prête que lorsque :

- chaque ligne de blocage de la Section 2.1 est résolue ou explicitement infirmée ;
- chaque élément non bloquant lié à la version est associé à un problème ou marqué comme terminé ;
- les rapports de conformité mentionnent la même révision du corpus de conformité estampillée ;
- les notes de version citent le commit de la spécification, la révision du corpus, les versions des paquets et les reports (deferrals) connus ;
- les simulations de paquets (package dry-runs) et les flux de travail de version (release workflows) réussissent ;
- les registres publics et la vérification des artefacts prouvent que les bits publiés correspondent à la version candidate prévue.
