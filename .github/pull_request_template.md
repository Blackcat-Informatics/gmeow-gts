<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
## Summary

- what changed
- why it changed
- which engine(s) it affects (Rust / Python / Go / TypeScript / Smalltalk / Kotlin / spec / corpus)

## Validation

- [ ] `pre-commit run --all-files` (formatting, SPDX headers, secrets)
- [ ] Tests pass for each affected engine:
  - [ ] `cd rust && cargo test` (and `cargo fmt --check` + `cargo clippy`)
  - [ ] `cd go && go test ./...` (and `go vet` + `golangci-lint`)
  - [ ] `cd ts && npm ci && npm test` (and `npm run lint`)
  - [ ] `cd python && uv run pytest` (and `uv run ruff check` + `uv run mypy`)
  - [ ] `docker build -t gmeow-gts-smalltalk smalltalk && docker run --rm -v "$PWD:/workspace" --entrypoint /bin/sh gmeow-gts-smalltalk -lc 'sh /workspace/smalltalk/scripts/run-tests.sh'`
  - [ ] `cd kotlin && gradle test detekt`
- [ ] A v2 format change updates the affected normative parts and [v2 conformance cases](../vectors/v2/README.md).
- [ ] For released-engine behavior changes, the generated corpus reproduces and affected engines agree.

- [ ] Docs / spec updated if behaviour, flags, or the wire format changed
- [ ] If the quality-budget baseline increased: PR has the
      `quality-budget-baseline-increase` label or an updated architecture-review note
