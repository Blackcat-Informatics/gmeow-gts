# Vectors of PF-stream

| vector | outcome | notes |
|---|---|---|
| `stream-pass` | built and matches | a segment of the profile stream with 60 commits and parity frames at positions 4213 and 8427 (12568601 bytes, listed in ABSENT.md): its recorded read has unprotected_tail as the one finding |
| `stream-pass-prefixes` | built and matches | the first prefix of stream-pass, its 12-byte header; the other prefixes (a sample of the item boundaries: every 397th, the three around each parity frame and the ends) are not stored |
| `stream-interval-boundary` | built and matches |  |
| `stream-interval` | built and matches | a commit interval of 201 frames: the finding is at the 201st frame of the commit interval, position 201 |
| `stream-interval-bad-commit` | built and matches |  |
| `stream-parity-missing` | built and matches |  |
| `stream-parity-misplaced` | built and matches | stream-pass with the parity frame of the first span moved two frames later: stream_parity_missing at the item that reaches the span (position 4212), count 1, beside parity_damaged (the text says parity_stale) |
| `stream-parity-damaged` | built and matches |  |
| `stream-no-parity` | built and matches |  |
| `stream-parity-last-item` | built and matches |  |
| `pf-example-bytes` | built and matches |  |
| `stream-prefix-commit-boundary` | built and matches | stream-pass cut after the commit that ends interval 1 (200127 bytes): the commit is valid in the prefix; the cuts after the commits that end intervals 21 and 60 are not stored |
| `stream-heal-pending-full` | built and matches | a commit whose stored identifier is 32 bytes 00, healed by the frame after it |
| `stream-heal-pending-prefix` | built and matches | the file of stream-heal-pending-full cut after the commit: the premise of the claim does not hold; the findings are those of the prefix as a file |
| `stream-heal-pending-by-parity` | built and matches | a commit that reaches the end of the first span, its stored identifier damaged, healed by the parity frame of the span (the input is listed in ABSENT.md) |
| `stream-concat-two` | built and matches | the byte concatenation of two files of stream-pass: no profile_violation; the second segment has lineage_begins; each segment has an unprotected_tail of the length of that of stream-pass at its own offset (PF-stream-7) |
| `stream-stream-read` | built and matches | stream-pass read with access forward at item boundaries (a sample: every 397th, and the three around each parity frame and the ends; all 12000 would read 12 MB each): no profile_violation at any of them (PF-stream-7, PF-stream-10) |

Totals: built and matches: 17 (17 entries).
