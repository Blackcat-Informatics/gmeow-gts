# Vectors of PF-stream

| vector | outcome | notes |
|---|---|---|
| `stream-pass` | built and matches | a segment of the profile stream with 60 commits and parity frames at positions 4213 and 8427 (12568601 bytes, listed in ABSENT.md): its recorded read has unprotected_tail as the one finding |
| `stream-pass-prefixes` | built and matches | the prefixes of stream-pass, a segment of the profile stream, that end at 43 item boundaries: items 0, 1 and the last (3), the parity frame and the two items on each side of it (items 4211 to 4215 and 8425 to 8429, 10 in all) and every 397th item (30); a reader of the profile reports no profile_violation at any of them, every commit that a prefix holds being valid in it; a prefix of at most 1 MiB is stored, a larger one is named by the file stream-pass.bin (listed in ABSENT.md), its length and its SHA-256, and the last is stream-pass.bin itself |
| `stream-interval-boundary` | built and matches |  |
| `stream-interval` | built and matches | a commit interval of 201 frames: the finding is at the 201st frame of the commit interval, position 201 |
| `stream-interval-bad-commit` | built and matches |  |
| `stream-parity-missing` | built and matches |  |
| `stream-parity-misplaced` | built and matches | stream-pass with the parity frame of span 0 moved two frames later (position 4215): kept as written, its link is broken and its anchor is not its predecessor, so it is parity_damaged; with its anchor and identifier made over its new predecessor (case anchor-rewritten) it is parity_stale; in both, stream_parity_missing at 4212, count 1 (each input is an edit of stream-pass.bin, stated with its length and SHA-256) |
| `stream-parity-damaged` | built and matches |  |
| `stream-no-parity` | built and matches |  |
| `stream-parity-last-item` | built and matches |  |
| `pf-example-bytes` | derived from the text | the deterministic encoding of the profile_violation findings of the worked examples of PF-stream (check 2, stream_parity_missing, at position 40 with count 1, 24 bytes) and of PF-container (check 11, container_dangling_auxiliary, with no position and count 1, 22 bytes); plain values, no input file |
| `stream-prefix-commit-boundary` | built and matches | stream-pass cut immediately after the commit that ends commit interval 1, after the one that ends interval 21 and after the one that ends interval 60: every commit that a prefix holds is valid in it, and a reader of the profile reports no profile_violation and no uncommitted_tail; a cut above 1 MiB is named by stream-pass.bin (listed in ABSENT.md), its length and its SHA-256, and the cut after interval 60 is stream-pass.bin itself |
| `stream-heal-pending-full` | built and matches | a commit whose stored identifier is 32 bytes 00, healed by the frame after it |
| `stream-heal-pending-prefix` | built and matches | the file of stream-heal-pending-full cut after the commit: the premise of the claim does not hold; the findings are those of the prefix as a file |
| `stream-heal-pending-by-parity` | built and matches | a commit that reaches the end of the first span, its stored identifier damaged, healed by the parity frame of the span (the input is listed in ABSENT.md) |
| `stream-concat-two` | built and matches | the byte concatenation of two files of stream-pass: no profile_violation; the second segment has lineage_begins; each segment has an unprotected_tail of the length of that of stream-pass at its own offset (PF-stream-7) |
| `stream-stream-read` | built and matches | stream-pass read with access forward at item boundaries (a sample: every 397th, and the three around each parity frame and the ends; all 12000 would read 12 MB each): no profile_violation at any of them (PF-stream-7, PF-stream-10) |

Totals: built and matches: 16; derived from the text: 1 (17 entries).
