<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS licensing backtranslation record

An independent model read the entire [Chinese licensing explanation](./docs/i18n/zh-Hans/LICENSING.md)
and translated it into English without reading the English original, linked licenses,
repository history or other project material. It freshly read the current source in full.
This is a model backtranslation and wording comparison, not human, native-speaker or
legal review. The explanatory document remains an unreviewed draft; the complete
licenses govern recipients' rights and obligations.

- Source read: `2026-10-01T15:14:52.939872+00:00`.
- Translation completed: `2026-10-01T15:17:53.732817+00:00`.
- Translated source: `docs/i18n/zh-Hans/LICENSING.md` (3,909 UTF-8 bytes).
- Translated-input SHA-256: `64a2972f49ed7c2a4f67efbbacb0353ec68c713d3eb7c84b3f1a1d0ae05709d0`.
- Unchanged [bilingual license](./LICENSE-MULAN) SHA-256:
  `eb7a1d713eb919b146787629e22e4c975cb701f529a65d4d7e0fcd417558bf1c`.

After translation, four relative links in the source were corrected to reach the
root MIT, Apache, Mulan and contribution documents from the Chinese document's
location. Reversing those four URL substitutions reproduces the translated input
byte for byte; no explanatory prose changed. The current source is 3,937 bytes,
SHA-256 `ce08aa982b08c888e334c6566593d4129a4265da12148e4c20f516c4c8a326c9`.

## Comparison

The repository comparison checked the returned text against the explanatory
English source and the unchanged bilingual MulanPSL-2.0 text. The contributor-to-recipient
patent grant, affiliates and litigation triggers, termination date, language precedence,
recipient-copy duty and notice-use exception are explicit in the Chinese source and
preserved in the backtranslation. The source keeps existing documentation and foreign
material grants separate from the first-party licensing choices.
The incoming open-source grant and the independent commercial-relicensing
permission are also distinguished. The later trademark section uses a shorter general
statement; read it with the explicit notice-use exception in the earlier Mulan paragraph
and with the complete licenses. The translation records its broad wording without
adding a new trademark policy. File-level scope and contribution procedures remain
specified by SPDX identifiers, `REUSE.toml` and `CONTRIBUTING.md`.

The translator's source-only findings do not independently verify licenses, package
contents, trademarks, contribution authority or distribution receipts. The comparison
above is a separate repository check. A change to the Chinese source requires updating
this record and its source fingerprint.

## Complete independent backtranslation

The source's SPDX metadata is recorded by its source file. The full explanatory prose
follows; heading levels and link paths are adjusted for this record's location.

### Licensing

> An informational Chinese translation of [`LICENSING.md`](./LICENSING.md). The English original governs the licensing explanation; where the Chinese and English wording of the MulanPSL-2.0 license itself is inconsistent, the Chinese text governs under its Article 6. This explanation has not undergone human legal review; the [independent model backtranslation record](./LICENSING-BACKTRANSLATION.md) is set out separately.

Blackcat Informatics® Inc. provides the first-party GTS implementation code, build and packaging tools, GTS specification, and frozen conformance test vectors under **MIT OR Apache-2.0 OR MulanPSL-2.0**, with recipients free to choose any one. The copyright holder may also separately grant commercial or proprietary licenses; commercial licensing is not a fourth open-source license.

#### Open-source Terms

| License | Text |
|---|---|
| **MIT** | [`LICENSE-MIT`](./LICENSE-MIT) |
| **Apache License 2.0** | [`LICENSE-APACHE`](./LICENSE-APACHE) |
| **Mulan Permissive Software License, Version 2** | [`LICENSE-MULAN`](./LICENSE-MULAN) |

The licensing choice is expressed as:

```text
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
```

This grant covers the applicable first-party materials in all implementations and wrappers; for the specific scope, see each file's SPDX identifier and the annotations in `REUSE.toml`. Other documents and generated reports retain the existing licenses in those identifiers or annotations. Third-party dependencies, reference materials, and the standard declaration wording required for IETF submissions retain their original terms and are not relicensed by this grant. Their copyright and license notices must be retained upon distribution.

The metadata of the Python distribution package declares both the three-way licensing choice for the implementation code and the retained MIT-or-Apache licensing choice of the embedded README. That license expression describes the archive contents and does not change the existing licensing grant for documents.

MulanPSL-2.0 is a bilingual license. **Under Article 6, the Chinese and English versions have equal legal effect; if they conflict or are inconsistent, the Chinese version governs.** Article 2 provides that each contributor grants you a patent license in accordance with its terms. If you or your affiliated entities directly or indirectly initiate patent-infringement litigation concerning this software or contributions within it (including counterclaims or crossclaims), or other actions to enforce patent rights, the patent license granted to you for this software by this license terminates as of the date that action is initiated; the specific scope and conditions are governed by the full text of Article 2. Article 3 grants no trademark license, except for use necessary to fulfill the notice obligations in Article 4. Article 4 requires that recipients be provided with a copy of the license and that the copyright, patent, trademark, and disclaimer notices in the software be retained. The committed bilingual text is pinned by SHA-256 `eb7a1d713eb919b146787629e22e4c975cb701f529a65d4d7e0fcd417558bf1c`.

Unless otherwise stated, contributions intentionally submitted for inclusion in the applicable first-party code, tools, specification, and vectors are provided under MIT OR Apache-2.0 OR MulanPSL-2.0, with no additional terms attached to that open-source grant. For the contribution process and separate authorization for commercial relicensing, see the “Contributions” section below. Contributions to other documents retain the license declared by the document.

#### Proprietary / Commercial Licensing

In addition to the open-source licenses, the copyright holder may still separately license materials that it owns. For commercial licensing, contact **licensing@blackcatinformatics.ca**.

#### Trademarks

“Blackcat Informatics®” is a registered trademark of Blackcat Informatics® Inc. The open-source licenses grant no trademark rights; see Article 6 of Apache License 2.0 and Article 3 of MulanPSL-2.0. Use solely for reference is permitted, for example “compatible with GTS”; endorsement or origin must not be implied.

#### Contributions

Contributions are accepted under the applicable open-source terms above and, under the project's CLA, provide separate authorization permitting additional commercial or proprietary licensing. Whether a contributor license agreement must be signed is determined by the project's contribution process; significant contributions may be subject to this requirement before merging. See [`CONTRIBUTING.md`](./CONTRIBUTING.md).

#### Copyright Notice

Copyright © 2026 Blackcat Informatics® Inc. All rights are reserved except those expressly granted by the applicable license. This adjustment to first-party licensing does not change the package version, wire-format version, or any bytes of the conformance test vectors.
