# PHP Packagist Publication

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

The PHP Composer package name is `blackcatinformatics/gmeow-gts`. Public
Packagist currently returns 404 for that package name, so it is available as of
2026-06-22. The lower-case vendor spelling is compatible with Packagist package
name rules.

Public Packagist expects `composer.json` at the package repository root. This
repository keeps the PHP wrapper under `php/`, and the maintainer decision for
this package is monorepo-only: do not create or publish a generated mirror
repository. The publication path is therefore a generated package-root commit
inside this repository.

## Package Root

Generate the Composer-facing root from the monorepo checkout:

```sh
bash scripts/package_php_packagist_root.sh dist/php-packagist-root
```

The generated root contains only:

- `composer.json`
- `README.md`
- `LICENSE-MIT`
- `LICENSE-APACHE`
- `src/`
- `tests/`

It intentionally excludes monorepo-only files, `php/Dockerfile`, and native
`libgts` binaries. Composer users must install `libgts` separately and enable
PHP FFI at runtime.

## Dry Run

Run the normal wrapper dry-run before creating a publication tag:

```sh
bash scripts/package_dry_run_wrappers.sh
```

The PHP portion validates `php/composer.json`, generates the package root,
validates the generated root, installs it into a temporary Composer project as
a path repository, and runs a PHP FFI smoke test against `libgts`.

## Release Tag

Packagist stable versions are derived from VCS tags, so do not add a committed
`version` field to `composer.json`. Use a semantic version tag such as `0.9.4`
or `v0.9.4` on the generated package-root commit. The existing language-prefixed
tags in this repository are useful for other ecosystems, but `php-v0.9.4` is
not the tag shape Packagist documents for stable Composer versions.

One way to create the publication commit without changing the main worktree is:

```sh
version=0.9.4
stage="$(mktemp -d)"

bash scripts/package_php_packagist_root.sh "${stage}/package"
git worktree add --detach "${stage}/worktree" HEAD
git -C "${stage}/worktree" switch --orphan php-packagist
git -C "${stage}/worktree" rm -rf .
cp -R "${stage}/package/." "${stage}/worktree/"
git -C "${stage}/worktree" add .
git -C "${stage}/worktree" commit -m "Prepare PHP Packagist package ${version}"
git -C "${stage}/worktree" tag "${version}"
git -C "${stage}/worktree" push origin HEAD:refs/heads/php-packagist "${version}"
git worktree remove "${stage}/worktree"
```

Submit `https://github.com/Blackcat-Informatics/gmeow-gts` to Packagist after
the tag is pushed. The submitted package should be
`blackcatinformatics/gmeow-gts`; Packagist will read the package metadata from
the generated package-root commit referenced by the stable tag.

## Auto-Update

After submission, enable GitHub synchronization from the Packagist account that
is connected to this GitHub organization. If automatic GitHub sync is not
available, configure a manual GitHub webhook for push events using Packagist's
documented endpoint:

```text
https://packagist.org/api/github?username=PACKAGIST_USERNAME
```

Use the Packagist API token as the webhook secret.
