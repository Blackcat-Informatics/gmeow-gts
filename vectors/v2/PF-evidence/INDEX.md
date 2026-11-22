# Vectors of PF-evidence

| vector | outcome | notes |
|---|---|---|
| `evidence-pass` | built and matches | 10 blob frames and a final commit signed with K; K held and trusted |
| `evidence-pass-two-segments` | built and matches | evidence-pass and an appended segment with a correct lineage |
| `evidence-lineage` | built and matches | the second segment's lineage absent |
| `evidence-lineage-mismatch` | built and matches | the second segment's lineage names another head |
| `evidence-interval-pass` | built and matches | two runs of exactly 1024 frames, each followed by a signed commit |
| `evidence-interval` | built and matches | a run of 1025 frames before the first commit |
| `evidence-tail` | built and matches | evidence-pass with one blob frame appended after the commit |
| `evidence-unsigned` | built and matches | the final commit carries no signature |
| `evidence-unauthenticated-untrusted` | built and matches | K held, not trusted |
| `evidence-unauthenticated-not-held` | built and matches | inputs empty |
| `evidence-unauthenticated-bad-root` | built and matches | the final commit's root altered, its signature made over the altered frame |
| `evidence-algorithm` | built and matches | a second signature by J on the commit; K and J held and trusted |
| `evidence-algorithm-only` | built and matches | the final commit signed by J only |
| `evidence-suppress-pass` | built and matches | a suppress frame hiding one quad line before the final signed commit |
| `evidence-suppress-tail` | built and matches | a suppress frame appended after the commit |
| `evidence-suppress-other-segment` | built and matches | a second segment declaring no profile holding one unsigned suppress frame, then an unsigned commit |
| `evidence-worked-example` | built and matches | section 6: the two findings and their bytes (22 and 27), a commit at position 7 and a run of 1030 frames |
| `evidence-suppress-sealed` | built and matches | a second segment of registry version 2 with a key table and an unsigned sealed frame whose inner frame is a suppress frame (P8 6.7) |
| `evidence-lineage-previous-unreadable` | built and matches | the second segment's lineage is present and the header of the previous segment is unreadable: evidence_lineage, no lineage_mismatch |
| `evidence-interval-invalid-commit` | built and matches | 1024 frames, a commit with a wrong root, 1 more frame before the final signed commit: one evidence_commit_interval at the 1025-th frame, count 1026 |
| `evidence-check-seven-segment` | built and matches | two segments declaring the profile, an unauthenticated suppress frame in the first: check 7 once, in the second |
| `evidence-lineage-resync` | built and matches | a segment opened by a resync item with lineage absent, after a segment that declares the profile: evidence_lineage for that segment |
| `evidence-writer-filler-marks` | built and matches | H (profile evidence) T Q C with C cut so that 100 bytes of its last value are missing; a filler of 100 bytes, then a resync item with profile evidence whose marks hold the torn_tail mark over the bytes of C and the filler mark over [L, L + 100) (PF-evidence-13) |

Totals: built and matches: 23 (23 entries).
