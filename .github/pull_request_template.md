<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
## Summary

- what changed
- why it changed
- which engine(s) it affects (Rust / Python / Go / TypeScript / spec / corpus)

## Validation

- [ ] `pre-commit run --all-files` (formatting, SPDX headers, secrets)
- [ ] Tests pass for each affected engine:
  - [ ] `cd rust && cargo test` (and `cargo fmt --check` + `cargo clippy`)
  - [ ] `cd go && go test ./...` (and `go vet` + `golangci-lint`)
  - [ ] `cd ts && npm ci && npm test` (and `npm run lint`)
  - [ ] `cd python && uv run pytest` (and `uv run ruff check` + `uv run mypy`)
- [ ] If behaviour changed: the conformance corpus is updated and reproducible
      (`cd python && uv run python scripts/gen_vectors.py && git diff --exit-code vectors`)
- [ ] If behaviour changed: all four engines agree on the new corpus
- [ ] Docs / spec updated if behaviour, flags, or the wire format changed
