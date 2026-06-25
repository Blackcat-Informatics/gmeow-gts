<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Localization

GTS localization is maintained as an informative layer beside the English
documentation. English remains authoritative for protocol rules, wire format
definitions, conformance language, security policy, release policy, governance,
licensing, command names, API names, package names, machine-readable values, and
registry metadata.

Supported locales:

| Locale | Audience | Directory |
|---|---|---|
| `fr-CA` | Canadian French readers | [`fr-CA/`](./fr-CA/) |
| `zh-Hans` | Simplified Chinese readers | [`zh-Hans/`](./zh-Hans/) |

## Source Layout

Localized documents live under `docs/i18n/<locale>/` and mirror repository-root
source paths. For example:

| English source | French translation | Simplified Chinese translation |
|---|---|---|
| `README.md` | `docs/i18n/fr-CA/README.md` | `docs/i18n/zh-Hans/README.md` |
| `docs/GTS-SPEC.md` | `docs/i18n/fr-CA/docs/GTS-SPEC.md` | `docs/i18n/zh-Hans/docs/GTS-SPEC.md` |

Every localized Markdown file must declare its source path, locale, and status:

```markdown
<!-- i18n-source: README.md -->
<!-- i18n-locale: fr-CA -->
<!-- i18n-status: placeholder -->
```

Allowed status values are:

| Status | Meaning |
|---|---|
| `placeholder` | Locale landing page or untranslated stub. |
| `draft` | Partial translation that is not ready for drift enforcement. |
| `translated` | Complete translation that must preserve protected source literals. |
| `reviewed` | Complete translation reviewed by a fluent human or equivalent process. |

When one locale declares a source path, every supported locale must also declare
that source path. This keeps translation tranches paired.

## Translation Rules

Follow these rules for all localized documentation:

- Keep English as the normative source; localized files are informative.
- Link back to the English source near the top of each localized file.
- Use the canonical terms in [`GLOSSARY.md`](./GLOSSARY.md).
- Use bilingual first use for core protocol terms when it helps readers, such as
  "trame (frame)" or "帧 (frame)".
- Do not translate commands, flags, package names, paths, URLs, media types, API
  identifiers, diagnostic codes, JSON keys, enum/status values, profile names,
  RDF terms used as identifiers, or code blocks.
- Preserve code examples, registry snippets, and machine-readable contract
  values exactly unless an issue explicitly narrows an exception.

## Maintenance

Before translating a tranche:

1. Update [`GLOSSARY.md`](./GLOSSARY.md) if the tranche introduces new recurring
   terminology.
2. Add both locale files in the mirrored location.
3. Mark incomplete files as `draft` or `placeholder`.
4. Mark complete files as `translated` only when code fences and protected
   literals from the English source are intentionally preserved.
5. Run `just check-i18n`.

The localization guard lives at `scripts/check_i18n_docs.py`. It validates
metadata, paired locale coverage, source paths, code-fence balance, and protected
literal preservation for translated or reviewed files.
