<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-BENCHMARK-RELEASE-REPORT.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: translated -->

# Modèle de rapport de version du repère GTS

> Traduction informative de [`docs/GTS-BENCHMARK-RELEASE-REPORT.md`](../../../../docs/GTS-BENCHMARK-RELEASE-REPORT.md). Le document anglais demeure la source normative pour les intégrations, les fonctionnalités avancées, les profils optionnels, les données de référence, les exemples, les identifiants et les valeurs lisibles par machine. Cette traduction suit [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md) et reste informative.


Utilisez ce modèle pour les notes de version v1, la révision de la version candidate (release-candidate) et les preuves en annexe de l'article.
Générez le rapport rempli avec :

```bash
just bench-release
```

Pour une exécution de version candidate (release-candidate), utilisez tous les moteurs, au moins trois itérations, et effectuez un commit de l'artéfact généré dans le lot de preuves de version plutôt que dans cet arbre source :

```bash
python scripts/bench_release_suite.py \
  --engines rust,python,go,ts,smalltalk \
  --iterations 5 \
  --vectors vectors/01-minimal.gts,vectors/23-files-profile-tree.gts,vectors/25b-streamable-compacted.gts \
  --out-dir dist/benchmarks/v1.0-rc1 \
  --strict
```

L'exécuteur écrit :

- `release-benchmark-report.json` pour les preuves lisibles par machine ;
- `release-benchmark-report.md` pour les notes de version ou le texte en annexe ;
- les montages (fixtures) d'écriture et d'archivage déterministes utilisés par l'exécution ;
- les produits par moteur utilisés pour mesurer les chemins d'écriture, de pack et d'unpack.

Par défaut, l'exécuteur écrit un rapport complet même lorsque les moteurs sélectionnés échouent ou sont indisponibles. Utilisez `--strict` pour le filtrage (gating) de version candidate une fois que les lignes ayant échoué doivent bloquer la candidate.
## Métadonnées de version obligatoires

| Champ | Valeur |
|---|---|
| Version candidate | |
| Chemin du rapport généré | |
| Ligne de commande du runner | |
| Commit du dépôt | |
| Commit de la spécification GTS | |
| Blob de la spécification GTS | |
| Commit du corpus de conformité | |
| SHA-256 du manifeste du corpus | |
| Plateforme | |
| Processeur / mémoire | |
| Versions du runner | |
## Entrées de repère

| Type | Chemin | Octets | SHA-256 | Notes |
|---|---|---:|---|---|
| Vecteur de conformité | | | | lecture/repli |
| Vecteur de conformité | | | | lecture/repli |
| Fixture d'écriture | | | | entrée `from-nq` |
| Fixture d'archive | | | | entrée `pack`/`unpack` |
## Sommaire des temps d'exécution CLI

Utilisez les médianes pour les affirmations des notes de version. Conservez les lignes échouées ou ignorées dans le rapport afin que les moteurs indisponibles soient visibles plutôt qu'omis silencieusement.

| Moteur | Opération | Entrée | Itérations | Médiane ms | Min ms | Max ms | Preuve de sortie |
|---|---|---|---:|---:|---:|---:|---|
| Rust | read-info | | | | | | |
| Rust | fold | | | | | | |
| Rust | write-from-nq | | | | | | |
| Rust | pack | | | | | | |
| Rust | unpack | | | | | | |
| Python | read-info | | | | | | |
| Python | fold | | | | | | |
| Python | write-from-nq | | | | | | |
| Python | pack | | | | | | |
| Python | unpack | | | | | | |
| Go | read-info | | | | | | |
| Go | fold | | | | | | |
| Go | write-from-nq | | | | | | |
| Go | pack | | | | | | |
| Go | unpack | | | | | | |
| TypeScript | read-info | | | | | | |
| TypeScript | fold | | | | | | |
| TypeScript | write-from-nq | | | | | | |
| TypeScript | pack | | | | | | |
| TypeScript | unpack | | | | | | |
| Smalltalk | read-info | | | | | | |
| Smalltalk | fold | | | | | | |
| Smalltalk | write-from-nq | | | | | | |
| Smalltalk | pack | | | | | | |
| Smalltalk | unpack | | | | | | |
## Résumé de la mémoire de diffusion en continu

Les preuves de mémoire de diffusion en continu ne sont pas directement comparables au temps réel (wall time) de l'interface de ligne de commande (CLI). Citez-les séparément et nommez la méthode utilisée pour chaque moteur.

| Moteur | Méthode | Entrée | Écoulé | Mémoire de crête / preuves d'allocation | Notes |
|---|---|---|---:|---:|---|
| Python | full-reader materialization | | | | |
| Rust | `read_to_sink_from_reader` streaming fold | | | | |
| Go | `go test ./reader -bench ... -benchmem` | | | | |
| TypeScript | browser `foldStreamToSink` harness | | | | |
## Extrait des notes de version

Les repères pour `<release>` ont été exécutés sur `<platform>` au commit du dépôt `<repo_commit>`, au commit de la spécification `<spec_commit>` et au commit du corpus de conformité `<corpus_commit>`. Les temps médians de lecture/repli/écriture/empaquetage/dépaquetage sont répertoriés dans `<report path>`. Les preuves de mémoire diffusable en continu sont signalées séparément car l'assistant Rust signale le RSS de pointe du processus, le repère Go signale les métriques d'allocation d'exécution et la mémoire TypeScript du navigateur doit être capturée à partir du harnais de navigateur utilisé pour la version candidate.
